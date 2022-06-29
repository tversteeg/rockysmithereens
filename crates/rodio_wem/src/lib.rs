mod codebook;
mod error;
mod utils;

use std::{
    io::{Read, Seek, SeekFrom, Write},
    time::Duration,
    vec::IntoIter,
};

use bitvec::{field::BitField, order::Lsb0, view::BitView};
use byteorder::{LittleEndian, WriteBytesExt};
use error::WemError;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    error::{context, VerboseError},
    multi::many0,
    number::{
        complete::{u16, u32, u8},
        Endianness,
    },
    IResult,
};
use rodio::Source;

use crate::{codebook::CodebookLibrary, error::Result};

/// Decoder for an Wem file.
#[derive(Debug)]
pub struct WemDecoder {
    /// Size of the riff block.
    riff_size: u64,
    /// The fmt chunk.
    ///
    /// This is required to be one of the chunks.
    fmt: Fmt,
    /// The raw data in bytes.
    data: Vec<u8>,
}

impl WemDecoder {
    /// Attempts to decode the data as a wwise file containing vorbis.
    pub fn new(bytes: &[u8]) -> Result<WemDecoder> {
        // Get the endianness
        let (i, endianness) = parse_endianness_by_header(bytes)?;

        // Get the size of the riff block
        let (i, riff_size_u32) = context("endianness", u32(endianness))(i)?;
        let riff_size = riff_size_u32 as u64 + 8;

        // Verify the next block is "WAVE"
        let (i, _) = context("wave block", tag("WAVE"))(i)?;

        // Read the chunks
        let (_, chunks) = parse_chunks(i, endianness)?;

        // Extract the required chunks
        let fmt = chunks.fmt()?.clone();
        dbg!(&fmt);
        let data = chunks.into_data()?;

        // Setup the headers
        let ident_header = lewton::header::read_header_ident(&fmt.to_ident_packet()?).unwrap();
        let comment_header = lewton::header::read_header_comment(&empty_comment_packet()?).unwrap();
        let setup_header = lewton::header::read_header_setup(
            &create_setup_packet(endianness, &fmt, &data)?,
            fmt.channels as u8,
            (fmt.block_size_0, fmt.block_size_1),
        )
        .unwrap();

        Ok(Self {
            riff_size,
            data,
            fmt,
        })
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
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub setup_packet_offset: u32,
    pub first_audio_packet_offset: u32,
    pub uid: u32,
    pub block_size_0: u8,
    pub block_size_1: u8,
    pub sample_count: u32,
    pub mod_packets: bool,
}

impl Fmt {
    /// Create a fake vorbis identification header packet.
    pub fn to_ident_packet(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(30);

        // The packet type (ident header)
        bytes.write_u8(1)?;

        // Magic
        bytes.write("vorbis".as_bytes())?;

        // Vorbis version
        bytes.write_u32::<LittleEndian>(0)?;

        // Audio channels
        bytes.write_u8(self.channels as u8)?;
        // Audio sample rate
        bytes.write_u32::<LittleEndian>(self.sample_rate)?;

        // Maximum bitrate
        bytes.write_i32::<LittleEndian>(0)?;
        // Nominal bitrate
        bytes.write_i32::<LittleEndian>(self.avg_bytes_per_second as i32 * 8)?;
        // Minimum bitrate
        bytes.write_i32::<LittleEndian>(0)?;

        // Blocksizes
        bytes.write_u8(self.block_size_0 | (self.block_size_1 << 4))?;

        // Framing
        bytes.write_u8(1)?;

        Ok(bytes)
    }
}

/// A data chunk.
#[derive(Debug)]
pub enum Chunk {
    Fmt(Fmt),
    Data(Vec<u8>),
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
            b"fmt " => parse_fmt_chunk(i, endianness, size)?,
            b"data" => {
                let (i, data) = take(size)(i)?;

                (i, Self::Data(data.to_vec()))
            }
            _ => todo!(),
        })
    }

    /// The chunk size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Chunk::Fmt(Fmt { size, .. }) => *size,
            Chunk::Data(data) => data.len() as u32,
        }
    }
}

/// A trait to get a specific chunk from the list of chunks.
trait ChunkList {
    /// Get the fmt chunk.
    fn fmt(&'_ self) -> Result<&'_ Fmt>;

    /// Get the data chunk as it's bytes.
    fn into_data(self) -> Result<Vec<u8>>;
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

    fn into_data(self) -> Result<Vec<u8>> {
        for chunk in self {
            if let Chunk::Data(data) = chunk {
                return Ok(data);
            }
        }

        Err(WemError::MissingChunk("data".to_string()))
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
    data: &'a [u8],
    endianness: Endianness,
    size: u32,
) -> IResult<&'a [u8], Chunk, VerboseError<&'a [u8]>> {
    // Read a constant we will ignore
    let (i, _) = context("fmt chunk codec id", tag(b"\xFF\xFF"))(data)?;

    // Read the values
    let (i, channels) = context("fmt chunk channels", u16(endianness))(i)?;
    let (i, sample_rate) = context("fmt chunk sample rate", u32(endianness))(i)?;
    let (i, avg_bytes_per_second) =
        context("fmt chunk average bytes per second", u32(endianness))(i)?;
    let (i, block_align) = context("fmt chunk block align", u16(endianness))(i)?;
    let (_, bits_per_sample) = context("fmt chunk bits per sample", u16(endianness))(i)?;

    // Read the vorbis data
    let vorb_data = &data[0x18..];
    let (i, sample_count) = context("fmt vorbis chunk sample count", u32(endianness))(vorb_data)?;
    let (_, mod_signal) = context("fmt vorbis chunk mod signal", u32(endianness))(i)?;
    let mod_packets =
        mod_signal != 0x4A && mod_signal != 0x4B && mod_signal != 0x69 && mod_signal != 0x70;

    let i = &vorb_data[0x10..];
    let (i, setup_packet_offset) =
        context("fmt vorbis chunk setup packet offset", u32(endianness))(i)?;
    let (_, first_audio_packet_offset) = context(
        "fmt vorbis chunk first audio packet offset",
        u32(endianness),
    )(i)?;

    let i = &vorb_data[0x24..];
    let (i, uid) = context("fmt vorbis chunk uid", u32(endianness))(i)?;
    let (i, block_size_0) = context("fmt vorbis chunk block size 0", u8)(i)?;
    let (i, block_size_1) = context("fmt vorbis chunk block size 1", u8)(i)?;

    Ok((
        i,
        Chunk::Fmt(Fmt {
            size,
            channels,
            sample_rate,
            avg_bytes_per_second,
            block_align,
            bits_per_sample,
            sample_count,
            setup_packet_offset,
            first_audio_packet_offset,
            mod_packets,
            uid,
            block_size_0,
            block_size_1,
        }),
    ))
}

/// Create a fake vorbis comment header packet.
pub fn empty_comment_packet() -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    // The packet type (comment header)
    bytes.write_u8(3)?;

    // Magic
    bytes.write("vorbis".as_bytes())?;

    // Vendor
    let vendor = format!(
        "Converted by {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    bytes.write_u32::<LittleEndian>(vendor.len() as u32)?;
    bytes.write(vendor.as_bytes())?;

    // No loop count, so no comments
    bytes.write_u32::<LittleEndian>(0)?;

    // Framing
    bytes.write_u8(1)?;

    Ok(bytes)
}

/// Create a fake vorbis setup header packet.
pub fn create_setup_packet(endianness: Endianness, fmt: &Fmt, data: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    // The packet type (setup header)
    bytes.write_u8(5)?;

    // Magic
    bytes.write("vorbis".as_bytes())?;

    // Read the size
    let (i, size) =
        context("setup packet size", u16(endianness))(&data[(fmt.setup_packet_offset as usize)..])?;

    // Get the amount of codebooks
    let (mut i, codebook_count_minus_one) = context("setup packet codebook count", u8)(i)?;
    let codebook_count = codebook_count_minus_one + 1;
    dbg!(size, codebook_count);

    // Rewrite the codebooks
    let codebook_lib = CodebookLibrary::from_aotuv();
    for _ in 0..codebook_count {
        // Get the codebook index
        let id: u16 = i[0..2].view_bits::<Lsb0>()[0..10].load();
        dbg!(id);

        // Rewrite the codebook
        let (len, new_bytes) = codebook_lib.rebuild(id as usize)?;

        bytes.write(&new_bytes)?;

        // Move the input buffer to the bytes read
        // The two extra bytes are for the index
        i = &i[len + 2..];
    }

    // Framing
    bytes.write_u8(1)?;

    Ok(bytes)
}
