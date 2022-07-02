mod error;
mod utils;

use std::{
    fmt::{Debug, Formatter},
    io::{Cursor, Write},
};

use aes::{
    cipher::{AsyncStreamCipher, KeyIvInit},
    Aes256,
};
use cfb_mode::Decryptor;
pub use error::{ArchiveReadError, Result};
use flate2::read::ZlibDecoder;
use nom::{
    bytes::complete::take,
    error::{context, VerboseError},
    multi::count,
    number::complete::{be_u128, be_u16, be_u32},
    IResult,
};
use semver::Version;

/// Rocksmith decryption primitives.
const ARC_KEY: [u8; 32] =
    hex_literal::hex!("C53DB23870A1A2F71CAE64061FDD0E1157309DC85204D4C5BFDF25090DF2572C");
const ARC_IV: [u8; 16] = hex_literal::hex!("E915AA018FEF71FC508132E4BB4CEB42");

/// Parsed Playstation archive file.
#[derive(Clone)]
pub struct PlaystationArchive {
    /// Supported version of this archive format.
    version: Version,
    /// How the data is compressed.
    compression_type: CompressionType,
    /// Files in the archive.
    file_entries: Vec<FileEntry>,
    /// How big the file block is.
    block_size: BlockSize,
    /// The actual file data.
    data: Vec<u8>,
    /// How the paths of the archive are formatted.
    archive_flags: ArchiveFlags,
    /// Sizes of the blocks.
    block_sizes: Vec<u16>,
}

impl PlaystationArchive {
    pub fn parse(file: &[u8]) -> Result<Self> {
        log::debug!("parsing psarc file of {} bytes", file.len());

        let (i, magic) = parse_magic(file)?;
        if !magic {
            return Err(ArchiveReadError::UnrecognizedFile);
        }

        let (i, version) = parse_version(i)?;
        if version != Version::new(1, 4, 0) {
            return Err(ArchiveReadError::UnsupportedVersion);
        }

        let (i, compression_type_value) = parse_compression_type(i)?;
        let compression_type = CompressionType::try_from_u32(compression_type_value)?;

        let (i, table_of_content) = parse_toc(i)?;

        log::trace!("got {} entries", table_of_content.entry_count);

        let (i, block_size_value) = parse_block_size(i)?;
        let block_size = BlockSize::try_from_u32(block_size_value)?;

        log::trace!("got block size of {}", block_size.to_u32());

        let (_, archive_flags_value) = parse_archive_flags(i)?;
        let archive_flags = ArchiveFlags::try_from_u32(archive_flags_value)?;

        // Get all file entries from the table of content
        let file_entries = table_of_content.file_entries(archive_flags)?;

        // Skip the file entries part
        let blocks_offset = table_of_content.size() + 32;
        let i = &file[blocks_offset as usize..];

        // Calculate the amount of block sizes based on the size of the table of content
        let num_blocks = (table_of_content.length - blocks_offset) / 2;
        let (_, block_sizes) = parse_block_sizes(i, num_blocks as usize)?;

        log::trace!("got {} block sizes", block_sizes.len());

        let mut this = Self {
            version,
            compression_type,
            file_entries,
            block_size,
            data: file.to_vec(),
            archive_flags,
            block_sizes,
        };

        this.calculate_file_entry_sizes()?;

        this.parse_manifest()?;

        log::debug!("file succesfully parsed");

        Ok(this)
    }

    /// Read a file.
    pub fn read_file(&self, file_index: usize) -> Result<Vec<u8>> {
        let entry = self
            .file_entries
            .get(file_index)
            .ok_or(ArchiveReadError::FileDoesNotExist)?;

        if !entry.path.is_empty() {
            log::debug!("reading file '{}'", entry.path);
        }

        // Return the raw bytes when no compression is expected
        if self.compression_type == CompressionType::None
            || entry.input_length == entry.length as usize
        {
            return Ok(self.data
                [entry.offset as usize..entry.offset as usize + entry.length as usize]
                .to_vec());
        } else if self.compression_type == CompressionType::Lzma {
            // We don't support this compression type yet
            todo!();
        }

        // Setup a cursor that will write the result into the vector
        let mut result = Cursor::new(Vec::with_capacity(entry.length as usize));

        // Get a slice which will be data for this entry
        let all_block_bytes =
            &self.data[entry.offset as usize..entry.offset as usize + entry.input_length];

        // Calculate how much blocks must be parsed
        let total_blocks = (entry.length as f32 / self.block_size.to_u32() as f32).ceil() as usize;

        log::trace!("reading {} blocks", total_blocks);

        // Extract all blocks
        let block_start = entry.index_list_size as usize;
        let mut chunk = all_block_bytes;
        for block_index in block_start..block_start + total_blocks {
            // Get the block size from the blocks
            let block_length = self.block_sizes.get(block_index).unwrap_or(&0);

            // Decrypt the blocks
            if *block_length == 0 {
                log::trace!("parsing uncompressed block {}", block_index);

                todo!()
            } else {
                // Try to find the magic bytes denoting the block as zlib compressed
                let zlib_magic = if chunk.len() >= 2 {
                    context("zlib magic", be_u16)(chunk)?.1
                } else {
                    // This value will always trigger the rest of the bytes to be copied
                    0
                };

                log::trace!("parsing block {}", block_index,);

                if zlib_magic == 0x78DA || zlib_magic == 0x7801 {
                    // Decode if compressed
                    let mut decoder = ZlibDecoder::new(chunk);
                    std::io::copy(&mut decoder, &mut result).map_err(|_| {
                        ArchiveReadError::Corrupt(
                            "could not copy decoded bytes to output".to_string(),
                        )
                    })?;

                    // Move the chunk further by how many bytes the decoder read
                    chunk = &chunk[decoder.total_in() as usize..];
                } else {
                    let mut block_size = self.block_size.to_u32() as usize;
                    if block_size > chunk.len() {
                        // Ensure that the block can't be read out of bounds
                        block_size = chunk.len();
                    }

                    log::trace!(
                        "found magic value 0x{:04X}, block is uncompressed with {} bytes",
                        zlib_magic,
                        block_size
                    );

                    // Write the rest
                    result.write(&chunk[..block_size]).map_err(|_| {
                        ArchiveReadError::Corrupt(
                            "could not copy uncompressed bytes to result buffer".to_string(),
                        )
                    })?;

                    chunk = &chunk[block_size..];
                }
            }
        }

        let string = result.into_inner();

        log::trace!("read total of {} bytes", string.len());

        // Verify the result size
        if string.len() != entry.length as usize {
            Err(ArchiveReadError::Corrupt(
                "read entry bytes doesn't match expected bytes".to_string(),
            ))
        } else {
            Ok(string)
        }
    }

    /// Read a file from a path.
    pub fn read_file_with_path(&self, path: &str) -> Result<Vec<u8>> {
        log::debug!("reading file with path '{}'", path);

        let index = self
            .index_for_path(&path)
            .ok_or(ArchiveReadError::PathNotFound(path.to_string()))?;

        self.read_file(index)
    }

    /// Read file as a string based on the Rocksmith path.
    pub fn read_rs_file(&self, path: &str, extension: &str) -> Result<Vec<u8>> {
        log::debug!("reading file with rs path '{}'", path);

        let searchable_path = format!(
            "{}.{}",
            path.split(':').last().expect("malformed path"),
            extension
        );
        let index = self
            .index_for_path_ending_with(&searchable_path)
            .ok_or(ArchiveReadError::PathNotFound(searchable_path))?;

        self.read_file(index)
    }

    /// Read a file as a string.
    pub fn read_file_as_string(&self, file_index: usize) -> Result<String> {
        let bytes = self.read_file(file_index)?;
        String::from_utf8(bytes)
            .map_err(|_| ArchiveReadError::Corrupt("could not convert bytes to utf-8".to_string()))
    }

    /// Read file as a string based on the Rocksmith path.
    pub fn read_rs_file_as_string(&self, path: &str, extension: &str) -> Result<String> {
        let bytes = self.read_rs_file(path, extension)?;
        String::from_utf8(bytes)
            .map_err(|_| ArchiveReadError::Corrupt("could not convert bytes to utf-8".to_string()))
    }

    /// Get the index for a file path.
    pub fn index_for_path(&self, path: &str) -> Option<usize> {
        self.file_entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.path == path)
            .map(|(i, _)| i)
    }

    /// Get the index for a file path.
    pub fn index_for_path_ending_with(&self, path: &str) -> Option<usize> {
        self.file_entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.path.ends_with(path))
            .map(|(i, _)| i)
    }

    /// All file paths as an iterator.
    pub fn paths_iter(&'_ self) -> impl Iterator<Item = &'_ String> {
        self.file_entries.iter().map(|entry| &entry.path)
    }

    /// All enumerated file paths filtered by extension as an iterator.
    pub fn enumerated_file_paths_by_extension_iter<'b>(
        &'b self,
        extension: &'b str,
    ) -> impl Iterator<Item = (usize, &'b String)> + 'b {
        self.file_entries
            .iter()
            .enumerate()
            .filter_map(move |(i, entry)| entry.path.ends_with(extension).then(|| (i, &entry.path)))
    }

    /// Amount of files in the archive.
    pub fn len(&self) -> usize {
        self.file_entries.len()
    }

    /// Whether there are any files in the archive.
    pub fn is_empty(&self) -> bool {
        self.file_entries.is_empty()
    }

    /// Fill the entries with the lines from the manifest.
    fn parse_manifest(&mut self) -> Result<()> {
        log::debug!("reading manifest");

        // Convert the lines to a vector of strings
        std::iter::once("manifest.txt")
            .chain(self.read_file_as_string(0)?.lines())
            .enumerate()
            .for_each(|(i, line)| self.file_entries[i].path = line.to_string());

        Ok(())
    }

    /// Fill the file entry sizes with the calculated total size.
    pub fn calculate_file_entry_sizes(&mut self) -> Result<()> {
        // Calculate the input lengths for the every but the last item
        let next_offsets = self
            .file_entries
            .iter()
            .skip(1)
            .map(|entry| entry.offset)
            .collect::<Vec<_>>();

        self.file_entries
            .iter_mut()
            .zip(next_offsets)
            .for_each(|(mut entry, next_offset)| {
                entry.input_length = (next_offset - entry.offset) as usize;
            });

        // Calculate the input length for the last item
        if let Some(mut last_entry) = self.file_entries.last_mut() {
            last_entry.input_length = self.data.len() - last_entry.offset as usize;
        }

        Ok(())
    }
}

impl Debug for PlaystationArchive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaystationArchive")
            .field("version", &self.version)
            .field("compression_type", &self.compression_type)
            .field("file_entries", &self.file_entries)
            .field("block_size", &self.block_size)
            .field("archive_flags", &self.archive_flags)
            .field("block_sizes", &self.block_sizes)
            .finish()
    }
}

/// How the archive is compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressionType {
    None,
    Zlib,
    Lzma,
}

impl CompressionType {
    /// Parse the value from the archive header.
    pub fn try_from_u32(value: u32) -> Result<Self> {
        match value {
            0x00000000 => Ok(CompressionType::None),
            0x7A6C6962 => Ok(CompressionType::Zlib),
            0x6C7A6D61 => Ok(CompressionType::Lzma),
            _ => Err(ArchiveReadError::Corrupt(
                "unrecognized compression type".to_string(),
            )),
        }
    }
}

/// How the paths of the archive are formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveFlags {
    /// The paths won't have slash at the start of every line, everything is accessed as if the
    /// archive is a directory.
    Relative,
    /// All paths are case insensitive.
    IgnoreCase,
    /// All paths start with a slash.
    Absolute,
    /// TOC is encrypted.
    Encrypted,
}

impl ArchiveFlags {
    /// Parse the value from the archive header.
    pub fn try_from_u32(value: u32) -> Result<Self> {
        match value {
            0 => Ok(ArchiveFlags::Relative),
            1 => Ok(ArchiveFlags::IgnoreCase),
            2 => Ok(ArchiveFlags::Absolute),
            4 => Ok(ArchiveFlags::Encrypted),
            _ => Err(ArchiveReadError::Corrupt(
                "unrecognized archive flags".to_string(),
            )),
        }
    }
}

/// How big each data block is in bytes.
#[derive(Debug, Clone, Copy)]
enum BlockSize {
    U16,
    U24,
    U32,
}

impl BlockSize {
    /// Parse the value from the archive header.
    pub fn try_from_u32(value: u32) -> Result<Self> {
        match value {
            65536 => Ok(BlockSize::U16),
            16777216 => Ok(BlockSize::U24),
            4294967295 => Ok(BlockSize::U32),
            _ => Err(ArchiveReadError::Corrupt(
                "unregular block size".to_string(),
            )),
        }
    }

    /// Convert the blocksize to it's number representation.
    pub fn to_u32(self) -> u32 {
        match self {
            BlockSize::U16 => 65536,
            BlockSize::U24 => 16777216,
            BlockSize::U32 => 4294967295,
        }
    }
}

/// Archive table of content data.
#[derive(Debug, Clone)]
struct TableOfContent<'a> {
    length: u32,
    entry_size: u32,
    entry_count: u32,
    data: &'a [u8],
}

impl<'a> TableOfContent<'a> {
    /// Get all file entries.
    pub fn file_entries(&self, flags: ArchiveFlags) -> Result<Vec<FileEntry>> {
        // If the archive flag is set to encrypted we'll have to decrypt the data
        let mut i = self.decrypt(flags)?;

        (0..self.entry_count)
            .map(|_| {
                let (i_ref, file_entry) = parse_file_entry(&i)?;
                i = i_ref.into();

                Ok(file_entry)
            })
            .collect()
    }

    /// Decrypt the TOC if the archive flag is set to encrypted.
    pub fn decrypt(&self, flags: ArchiveFlags) -> Result<Vec<u8>> {
        // Skip the first bytes that have already been parsed
        let (i, _) = take(8usize)(self.data)?;

        // Take the exact bytes for the TOS
        let (_, bytes) = context("table of content bytes", take(self.size()))(i)?;
        let mut bytes = bytes.to_vec();

        // Decrypt the TOS if the Rocksmith encryption flags have been set
        if flags == ArchiveFlags::Encrypted {
            // Decrypt the TOS
            let decryptor = Decryptor::<Aes256>::new(&ARC_KEY.into(), &ARC_IV.into());

            decryptor.decrypt(&mut bytes);
        }

        if bytes.len() != self.size() as usize {
            Err(ArchiveReadError::Corrupt(
                "table of content input size doesn't match decrypted size".to_string(),
            ))
        } else {
            Ok(bytes)
        }
    }

    /// Get the true amount of bytes for the TOC.
    pub fn size(&self) -> u32 {
        self.entry_size * self.entry_count
    }
}

/// Single file entry in the archive.
#[derive(Clone)]
struct FileEntry {
    /// Will be set after manifest is parsed.
    path: String,
    /// Index in the block list size.
    index_list_size: u32,
    /// Uncompressed size.
    length: u64,
    /// Byte offset in whole file.
    offset: u64,
    /// Total bytes of the whole file, will be filled when the block sizes are known.
    input_length: usize,
}

impl Debug for FileEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileEntry")
            .field("path", &self.path)
            .field("index_list_size", &self.index_list_size)
            .field("length", &self.length)
            .field("offset", &self.offset)
            .field("input_length", &self.input_length)
            .finish()
    }
}

/// Parse the magic number at the beginning of the header.
fn parse_magic<'a>(i: &'a [u8]) -> IResult<&'a [u8], bool, VerboseError<&'a [u8]>> {
    let (i, magic) = context("magic", be_u32)(i)?;

    Ok((i, magic == 0x50534152))
}

/// Parse major and minor version numbers.
fn parse_version<'a>(i: &'a [u8]) -> IResult<&'a [u8], Version, VerboseError<&'a [u8]>> {
    let (i, major) = context("major version", be_u16)(i)?;
    let (i, minor) = context("minor version", be_u16)(i)?;

    Ok((i, Version::new(major as u64, minor as u64, 0)))
}

/// Parse compression type.
fn parse_compression_type<'a>(i: &'a [u8]) -> IResult<&'a [u8], u32, VerboseError<&'a [u8]>> {
    context("compression type", be_u32)(i)
}

/// Parse the table of contents.
fn parse_toc<'a>(i: &'a [u8]) -> IResult<&'a [u8], TableOfContent<'a>, VerboseError<&'a [u8]>> {
    let (i, length) = context("table of contents length", be_u32)(i)?;
    let (i, entry_size) = context("table of contents entry size", be_u32)(i)?;
    let (i, entry_count) = context("table of contents entry count", be_u32)(i)?;

    let toc = TableOfContent {
        length,
        entry_size,
        entry_count,
        data: i,
    };

    Ok((i, toc))
}

/// Parse block size.
fn parse_block_size<'a>(i: &'a [u8]) -> IResult<&'a [u8], u32, VerboseError<&'a [u8]>> {
    context("block size", be_u32)(i)
}

/// Parse archive flags.
fn parse_archive_flags<'a>(i: &'a [u8]) -> IResult<&'a [u8], u32, VerboseError<&'a [u8]>> {
    context("archive flags", be_u32)(i)
}

/// Parse file entry.
fn parse_file_entry<'a>(i: &'a [u8]) -> IResult<&'a [u8], FileEntry, VerboseError<&'a [u8]>> {
    let (i, _name_digest_block) = context("file entry", be_u128)(i)?;

    let (i, index_list_size) = context("file entry index list size", be_u32)(i)?;

    let (i, length) = context("file entry length", utils::be_u40)(i)?;
    let (i, offset) = context("file entry offset", utils::be_u40)(i)?;

    let file_entry = FileEntry {
        index_list_size,
        length,
        offset,
        path: String::new(),
        input_length: 0,
    };

    Ok((i, file_entry))
}

/// Parse block sizes.
fn parse_block_sizes<'a>(
    i: &'a [u8],
    num_blocks: usize,
) -> IResult<&'a [u8], Vec<u16>, VerboseError<&'a [u8]>> {
    context("block_sizes", count(be_u16, num_blocks))(i)
}
