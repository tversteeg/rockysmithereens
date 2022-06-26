mod error;

use std::{
    io::{Read, Seek, SeekFrom},
    time::Duration,
    vec::IntoIter,
};

use error::WemError;
use lewton::inside_ogg::OggStreamReader;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    error::{context, VerboseError},
    multi::many0,
    number::{
        complete::{u16, u32},
        Endianness,
    },
    IResult,
};
use rodio::Source;

use crate::error::Result;

/// Decoder for an Wem file.
#[derive(Debug)]
pub struct WemDecoder {
    /// Size of the riff block.
    riff_size: u64,
    /// The fmt chunk.
    ///
    /// This is required to be one of the chunks.
    fmt: Fmt,
}

impl WemDecoder {
    /// Attempts to decode the data as a wwise file containing vorbis.
    pub fn new(data: &[u8]) -> Result<WemDecoder> {
        // Get the endianness
        let (i, endianness) = parse_endianness_by_header(data)?;

        // Get the size of the riff block
        let (i, riff_size_u32) = context("endianness", u32(endianness))(i)?;
        let riff_size = riff_size_u32 as u64 + 8;

        // Verify the next block is "WAVE"
        let (i, _) = context("wave block", tag("WAVE"))(i)?;

        // Read the chunks
        let (_, chunks) = parse_chunks(i, endianness)?;

        // Extract the required chunks
        let fmt = chunks.fmt()?.clone();

        Ok(dbg!(Self { riff_size, fmt }))
    }
}

impl Source for WemDecoder {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        todo!()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.fmt.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.fmt.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        // TODO
        None
    }
}

impl Iterator for WemDecoder {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        todo!()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

/// Fmt chunk data.
#[derive(Debug, Clone)]
pub struct Fmt {
    pub size: u32,
    pub channels: u16,
    pub sample_rate: u32,
    pub avg_bytes_per_second: u32,
}

/// A data chunk.
#[derive(Debug)]
pub enum Chunk {
    Fmt(Fmt),
    Data { size: u32 },
}

impl Chunk {
    /// Parse with nom the bytes for this chunk.
    pub fn parse<'a>(
        i: &'a [u8],
        endianness: Endianness,
    ) -> IResult<&'a [u8], Self, VerboseError<&'a [u8]>> {
        // Get the chunk type string
        let (i, chunk_type_bytes) = context("chunk type", take(4usize))(i)?;
        let chunk_type: &[u8; 4] = chunk_type_bytes
            .try_into()
            // Should never panic because nom should throw an error when the bytes can't be taken
            .unwrap();

        // Get the remaining size of this chunk
        let (i, size) = context("chunk size", u32(endianness))(i)?;

        dbg!(size);

        // Parse the chunk depending on the type
        Ok(match chunk_type {
            b"fmt " => {
                let (i, (channels, sample_rate, avg_bytes_per_second)) =
                    parse_fmt_chunk(i, endianness)?;

                (
                    i,
                    Self::Fmt(Fmt {
                        size,
                        channels,
                        sample_rate,
                        avg_bytes_per_second,
                    }),
                )
            }
            b"data" => (i, Self::Data { size }),
            _ => todo!(),
        })
    }

    /// The chunk size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Chunk::Fmt(Fmt { size, .. }) => *size,
            Chunk::Data { size } => *size,
        }
    }
}

/// A trait to get a specific chunk from the list of chunks.
trait ChunkList {
    /// Get the fmt chunk.
    fn fmt(&'_ self) -> Result<&'_ Fmt>;
}

impl ChunkList for Vec<Chunk> {
    fn fmt(&'_ self) -> Result<&'_ Fmt> {
        for chunk in self {
            if let Chunk::Fmt(fmt) = chunk {
                return Ok(fmt);
            }
        }

        Err(WemError::MissingChunk("fmt".to_string()))
    }
}

/// Parse header to get the endianness of the file.
///
/// `true` means it's little endian.
fn parse_endianness_by_header<'a>(
    i: &'a [u8],
) -> IResult<&'a [u8], Endianness, VerboseError<&'a [u8]>> {
    let (i, header) = context("RIFF/RIFX header", alt((tag("RIFF"), tag("RIFX"))))(i)?;

    Ok((
        i,
        if header == "RIFF".as_bytes() {
            Endianness::Little
        } else {
            Endianness::Big
        },
    ))
}

/// Parse chunks.
fn parse_chunks<'a>(
    i: &'a [u8],
    endianness: Endianness,
) -> IResult<&'a [u8], Vec<Chunk>, VerboseError<&'a [u8]>> {
    let mut chunks = Vec::new();

    // Keep track of the chunks by way of the reported sizes
    let mut chunk_offset = 0;

    while (chunk_offset as usize) < i.len() - 12 {
        // Parse the chunk
        let (_, chunk) = Chunk::parse(&i[chunk_offset as usize..], endianness)?;

        chunk_offset += chunk.size() + 8;

        chunks.push(chunk);
    }

    Ok((i, chunks))
}

/// Parse the fmt chunk.
fn parse_fmt_chunk<'a>(
    i: &'a [u8],
    endianness: Endianness,
) -> IResult<&'a [u8], (u16, u32, u32), VerboseError<&'a [u8]>> {
    // Read a constant we will ignore
    let (i, _) = context("fmt chunk codec id", tag(b"\xFF\xFF"))(i)?;

    // Read the values
    let (i, channels) = context("fmt chunk channels", u16(endianness))(i)?;
    let (i, sample_rate) = context("fmt chunk sample rate", u32(endianness))(i)?;
    let (i, avg_bytes_per_second) =
        context("fmt chunk average bytes per second", u32(endianness))(i)?;

    Ok((i, (channels, sample_rate, avg_bytes_per_second)))
}
