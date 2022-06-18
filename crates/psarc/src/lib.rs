mod error;
mod utils;

use aes::{
    cipher::{AsyncStreamCipher, KeyIvInit},
    Aes256,
};
use cfb_mode::Decryptor;
pub use error::{ArchiveReadError, Result};
use nom::{
    bytes::complete::take,
    error::{context, VerboseError},
    number::complete::{be_u128, be_u16, be_u32, be_u64},
    IResult,
};
use semver::Version;

/// Rocksmith decryption primitives.
const ARC_KEY: [u8; 32] =
    hex_literal::hex!("C53DB23870A1A2F71CAE64061FDD0E1157309DC85204D4C5BFDF25090DF2572C");
const ARC_IV: [u8; 16] = hex_literal::hex!("E915AA018FEF71FC508132E4BB4CEB42");

/// Parsed Playstation archive file.
#[derive(Debug, Clone)]
pub struct PlaystationArchive<'a> {
    /// Supported version of this archive format.
    version: Version,
    /// How the data is compressed.
    compression_type: CompressionType,
    /// Metadata for the archive.
    table_of_content: TableOfContent<'a>,
    /// How big the file block is.
    block_size: BlockSize,
    /// The actual file data.
    data: &'a [u8],
    /// How the paths of the archive are formatted.
    archive_flags: ArchiveFlags,
}

impl<'a> PlaystationArchive<'a> {
    pub fn parse(file: &'a [u8]) -> Result<Self> {
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

        let (i, archive_flags_value) = parse_archive_flags(i)?;
        let archive_flags = ArchiveFlags::try_from_u32(archive_flags_value)?;

        dbg!(table_of_content.file_entries(archive_flags));
        dbg!(table_of_content.length);
        dbg!(table_of_content.entry_size);
        todo!();

        Ok(Self {
            version,
            compression_type,
            table_of_content,
            block_size,
            data: i,
            archive_flags,
        })
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
        let (_, bytes) = context("table of content bytes", take(self.length))(i)?;
        let mut bytes = bytes.to_vec();

        // Decrypt the TOS if the Rocksmith encryption flags have been set

        if flags == ArchiveFlags::Encrypted {
            // Decrypt the TOS
            let decryptor = Decryptor::<Aes256>::new(&ARC_KEY.into(), &ARC_IV.into());

            decryptor.decrypt(&mut bytes);
        }

        Ok(bytes)
    }
}

/// Single file entry in the archive.
#[derive(Debug)]
struct FileEntry {
    name_digest: [u8; 16],
    index_list_size: u32,
    length: u64,
    offset: u64,
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
    };

    Ok((i, file_entry))
}
