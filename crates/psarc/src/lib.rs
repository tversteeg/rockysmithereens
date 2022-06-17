mod error;

pub use error::{ArchiveReadError, Result};
use nom::{
    error::{context, VerboseError},
    number::complete::{be_u16, be_u32},
    IResult,
};
use semver::Version;

/// Parsed Playstation archive file.
#[derive(Debug, Clone)]
pub struct PlaystationArchive<'a> {
    /// Supported version of this archive format.
    version: Version,
    /// How the data is compressed.
    compression_type: CompressionType,
    /// Metadata for the archive.
    table_of_content: TableOfContent,
    /// How big the file block is.
    block_size: u32,
    /// The actual file data.
    data: &'a [u8],
    /// How the paths of the archive are formatted.
    archive_flags: ArchiveFlags,
}

impl<'a> PlaystationArchive<'a> {
    pub fn parse(file: &[u8]) -> Result<Self> {
        let (i, magic) = parse_magic(file)?;
        if !magic {
            return Err(ArchiveReadError::UnrecognizedFile);
        }

        let (i, version) = parse_version(i)?;
        if version < Version::new(1, 4, 0) {
            return Err(ArchiveReadError::UnsupportedVersion);
        }

        Ok(Self {
            version,
            compression_type: todo!(),
            table_of_content: todo!(),
            block_size: todo!(),
            data: todo!(),
            archive_flags: todo!(),
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

/// How the paths of the archive are formatted.
#[derive(Debug, Clone, Copy)]
enum ArchiveFlags {
    /// The paths won't have slash at the start of every line, everything is accessed as if the
    /// archive is a directory.
    Relative,
    /// All paths are case insensitive.
    IgnoreCase,
    /// All paths start with a slash.
    Absolute,
}

/// Archive table of content data.
#[derive(Debug, Clone)]
struct TableOfContent {
    data: Vec<u8>,
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
