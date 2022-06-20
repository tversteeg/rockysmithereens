mod error;
mod utils;

use std::{
    fmt::{Debug, Formatter},
    io::{Cursor, Read, Write},
};

use aes::{
    cipher::{AsyncStreamCipher, KeyIvInit},
    Aes256,
};
use cfb_mode::Decryptor;
pub use error::{ArchiveReadError, Result};
use flate2::bufread::ZlibDecoder;
use nom::{
    branch::alt,
    bytes::complete::{take, take_till},
    combinator::{eof, opt, rest},
    error::{context, VerboseError},
    multi::count,
    number::complete::{be_u128, be_u16, be_u32, be_u64, le_u16},
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

        let (i, block_size_value) = parse_block_size(i)?;
        let block_size = BlockSize::try_from_u32(block_size_value)?;

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

        let mut this = Self {
            version,
            compression_type,
            file_entries,
            block_size,
            data: file.to_vec(),
            archive_flags,
            block_sizes,
        };

        this.parse_manifest()?;

        Ok(this)
    }

    /// Read a file.
    pub fn read_file(&self, file_index: usize) -> Result<Vec<u8>> {
        let entry = self
            .file_entries
            .get(file_index)
            .ok_or(ArchiveReadError::FileDoesNotExist)?;

        Ok(match self.compression_type {
            CompressionType::None => self.data
                [entry.offset as usize..entry.offset as usize + entry.length as usize]
                .to_vec(),
            CompressionType::Zlib => {
                let mut result = Cursor::new(Vec::with_capacity(entry.length as usize));

                let block_start = &self.data[entry.offset as usize..];

                let mut block_index = 0;
                let mut chunk = block_start;
                while result.position() < entry.length {
                    // Get the block size from the blocks
                    let block_length = self
                        .block_sizes
                        .get(entry.index_list_size as usize + block_index)
                        .unwrap_or_else(|| &0);

                    // Decrypt the blocks
                    if *block_length == 0 {
                        // If there's no block sizes available anymore use the default value
                        let (i, bytes) = context(
                            "raw block",
                            alt((
                                context("block size", take(self.block_size.to_u32())),
                                context("rest of the file", rest),
                            )),
                        )(chunk)?;
                        chunk = i;

                        result.write(bytes).map_err(|_| ArchiveReadError::Corrupt)?;
                    } else if chunk.len() >= 2 {
                        // Try to find the magic bytes denoting the block as zlib compressed
                        let (_, zlib_magic) = context("zlib magic", be_u16)(chunk)?;
                        if zlib_magic == 0x78DA || zlib_magic == 0x7801 {
                            // Take a slice the size of the block from the file
                            let (_, bytes) = context(
                                "compressed block",
                                alt((
                                    context("block size", take(self.block_size.to_u32())),
                                    context("rest of the file", rest),
                                )),
                            )(chunk)?;

                            // Decode if compressed
                            let mut decoder = ZlibDecoder::new(bytes);
                            std::io::copy(&mut decoder, &mut result)
                                .map_err(|_| ArchiveReadError::Corrupt)?;

                            // Move the chunk further by how many bytes the decoder read
                            chunk = &chunk[decoder.total_in() as usize..];
                        } else {
                            // No zlib magic header found, parse as raw bytes

                            // Take a slice the size of the block from the file
                            let (i, bytes) = context(
                                "raw block",
                                alt((
                                    context("block size", take(entry.length - result.position())),
                                    context("rest of the file", rest),
                                )),
                            )(chunk)?;
                            chunk = i;

                            result.write(bytes).map_err(|_| ArchiveReadError::Corrupt)?;
                        }
                    } else {
                        // Remaining chunk is only 1 or 2 bytes
                        result.write(chunk).map_err(|_| ArchiveReadError::Corrupt)?;
                    }

                    block_index += 1;
                }

                let string = result.into_inner();
                if string.len() != entry.length as usize {
                    return Err(ArchiveReadError::Corrupt);
                }

                string
            }
            CompressionType::Lzma => todo!(),
        })
    }

    /// Read file as a string based on the Rocksmith path.
    pub fn read_rs_file(&self, path: &str, extension: &str) -> Result<Vec<u8>> {
        let searchable_path = format!(
            "{}.{}",
            path.split(':').last().expect("malformed path"),
            extension
        );
        let index = self
            .index_for_path_ending_with(&searchable_path)
            .ok_or_else(|| ArchiveReadError::PathNotFound(searchable_path))?;

        self.read_file(index)
    }

    /// Read a file as a string.
    pub fn read_file_as_string(&self, file_index: usize) -> Result<String> {
        let bytes = self.read_file(file_index)?;
        String::from_utf8(bytes).map_err(|_| ArchiveReadError::Corrupt)
    }

    /// Read file as a string based on the Rocksmith path.
    pub fn read_rs_file_as_string(&self, path: &str, extension: &str) -> Result<String> {
        let bytes = self.read_rs_file(path, extension)?;
        String::from_utf8(bytes).map_err(|_| ArchiveReadError::Corrupt)
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

    /// Fill the entries with the lines from the manifest.
    fn parse_manifest(&mut self) -> Result<()> {
        // Convert the lines to a vector of strings
        std::iter::once("manifest.txt")
            .chain(self.read_file_as_string(0)?.lines())
            .enumerate()
            .for_each(|(i, line)| self.file_entries[i].path = line.to_string());

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
#[derive(Debug, Clone, Copy)]
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
            _ => Err(ArchiveReadError::Corrupt),
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
            _ => Err(ArchiveReadError::Corrupt),
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
            _ => Err(ArchiveReadError::Corrupt),
        }
    }

    /// Convert the blocksize to it's number representation.
    pub fn to_u32(&self) -> u32 {
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

        Ok(bytes)
    }

    /// Get the true amount of bytes for the TOC.
    pub fn size(&self) -> u32 {
        self.entry_size * self.entry_count
    }
}

/// Single file entry in the archive.
#[derive(Clone)]
struct FileEntry {
    name_digest: [u8; 16],
    /// Will be set after manifest is parsed.
    path: String,
    /// Index in the block list size.
    index_list_size: u32,
    /// Uncompressed size.
    length: u64,
    /// Byte offset in whole file.
    offset: u64,
}

impl Debug for FileEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileEntry")
            .field("path", &self.path)
            .field("index_list_size", &self.index_list_size)
            .field("length", &self.length)
            .field("offset", &self.offset)
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
    let (i, name_digest_block) = context("file entry", be_u128)(i)?;
    let name_digest = name_digest_block.to_be_bytes();

    let (i, index_list_size) = context("file entry index list size", be_u32)(i)?;

    let (i, length) = context("file entry length", utils::be_u40)(i)?;
    let (i, offset) = context("file entry offset", utils::be_u40)(i)?;

    let file_entry = FileEntry {
        name_digest,
        index_list_size,
        length,
        offset,
        path: String::new(),
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
