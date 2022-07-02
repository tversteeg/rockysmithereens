mod codebook;
mod error;
mod packet;
mod utils;

use std::{
    io::{Read, Seek, SeekFrom, Write},
    thread::panicking,
    time::{self, Duration},
    vec::IntoIter,
};

use bitvec::{field::BitField, order::Lsb0, prelude::BitVec, view::BitView};
use byteorder::{LittleEndian, WriteBytesExt};
use error::WemError;
use lewton::{
    audio::PreviousWindowRight,
    header::{CommentHeader, HeaderSet, IdentHeader, SetupHeader},
    samples::InterleavedSamples,
};
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
use packet::Packet;
use rodio::Source;

use crate::{
    codebook::CodebookLibrary,
    error::Result,
    utils::{log2, read, read_bool, read_write, read_write_bool, write},
};

/// Decoder for an Wem file.
pub struct WemDecoder {
    /// The fmt chunk.
    ///
    /// This is required to be one of the chunks.
    fmt: Fmt,
    /// The raw data in bytes.
    packets: Vec<Packet>,
    /// Index of the current packet.
    current_packet: usize,
    /// Decoding position.
    previous_window: PreviousWindowRight,
    /// Vorbis identification header.
    ident: IdentHeader,
    /// Vorbis setup header.
    setup: SetupHeader,
    /// Vorbis comment header.
    comment: CommentHeader,
    /// Current samples read.
    current_data: IntoIter<i16>,
    /// Whether we are done with this song.
    done: bool,
}

impl WemDecoder {
    /// Attempts to decode the data as a wwise file containing vorbis.
    pub fn new(bytes: &[u8]) -> Result<WemDecoder> {
        // Get the endianness
        let (i, endianness) = parse_endianness_by_header(bytes)?;

        // Get the size of the riff block
        let (i, riff_size_u32) = context("endianness", u32(endianness))(i)?;
        let _riff_size = riff_size_u32 as u64 + 8;

        // Verify the next block is "WAVE"
        let (i, _) = context("wave block", tag("WAVE"))(i)?;

        // Read the chunks
        let (_, chunks) = parse_chunks(i, endianness)?;

        // Extract the required chunks
        let fmt = chunks.fmt()?.clone();
        let data = chunks.into_data()?;

        // Setup the headers
        let ident = lewton::header::read_header_ident(&fmt.to_ident_packet()?)?;
        let comment = lewton::header::read_header_comment(&empty_comment_packet()?)?;

        let (setup_packet, mode_blockflag, mode_bits) =
            create_setup_packet(endianness, &fmt, &data)?;

        let setup = lewton::header::read_header_setup(
            &setup_packet,
            fmt.channels as u8,
            (fmt.block_size_0, fmt.block_size_1),
        )?;

        // Parse the data into packets
        let packets = packet::parse_into_packets(
            &data[(fmt.first_audio_packet_offset as usize)..],
            mode_blockflag,
            mode_bits,
        )?;

        let previous_window = PreviousWindowRight::new();

        let mut this = Self {
            fmt,
            packets,
            previous_window,
            ident,
            comment,
            setup,
            current_data: Vec::new().into_iter(),
            done: false,
            current_packet: 0,
        };

        // The first read initializes lewton
        this.read_packet()?;

        Ok(this)
    }

    /// Get the raw vorbis info.
    pub fn into_raw(self) -> (HeaderSet, Vec<Packet>) {
        ((self.ident, self.comment, self.setup), self.packets)
    }

    /// Read a packet.
    fn read_packet(&mut self) -> Result<()> {
        let audio: InterleavedSamples<_> = lewton::audio::read_audio_packet_generic(
            &self.ident,
            &self.setup,
            //&self.data[self.fmt.first_audio_packet_offset as usize..],
            &self.packets[self.current_packet].data,
            &mut self.previous_window,
        )?;

        self.current_data = audio.samples.into_iter();

        // Move to the next packet
        self.current_packet += 1;

        // We are done when we read all packets
        self.done = self.current_packet == self.packets.len();

        Ok(())
    }
}

impl Source for WemDecoder {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        let len = self.current_data.len();

        match (self.done, len > 0) {
            // A zero length means done
            (true, _) => Some(0),
            (false, true) => Some(len),
            // None means we are not done but don't have any bytes available
            (false, false) => None,
        }
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
        None
    }
}

impl Iterator for WemDecoder {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.done {
            None
        } else if let Some(sample) = self.current_data.next() {
            Some(sample)
        } else {
            self.read_packet()
                .ok()
                .map(|_| self.current_data.next())
                .flatten()
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.current_data.size_hint().0, None)
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
///
/// Also returns `mode_blockflag` and `mode_bits`.
pub fn create_setup_packet(
    endianness: Endianness,
    fmt: &Fmt,
    data: &[u8],
) -> Result<(Vec<u8>, Vec<bool>, u32)> {
    let mut bytes = BitVec::<_, Lsb0>::new();

    // The packet type (setup header)
    bytes.write_u8(5)?;

    // Magic
    bytes.write("vorbis".as_bytes())?;

    // Read the size
    let (i, _size) =
        context("setup packet size", u16(endianness))(&data[(fmt.setup_packet_offset as usize)..])?;

    // From now on we read individual bits
    let i = i.view_bits::<Lsb0>();

    // Get the amount of codebooks
    let (mut i, codebook_count_minus_one): (_, u16) = read_write(i, &mut bytes, 8);
    let codebook_count = codebook_count_minus_one + 1;

    // Rewrite the codebooks
    let codebook_lib = CodebookLibrary::from_aotuv();
    for _ in 0..codebook_count {
        // Get the codebook index
        let id: u16;
        (i, id) = read(i, 10);

        // Rewrite the codebook
        let new_bytes = codebook_lib.rebuild(id as usize)?;
        bytes.extend(new_bytes);
    }

    // Time domain transforms placeholder

    // Time count minus one
    write(0u8, &mut bytes, 6);
    // Dummy time value
    write(0u16, &mut bytes, 16);

    // Rebuild floors
    let (mut i, floor_count_minus_one): (_, u8) = read_write(i, &mut bytes, 6);
    let floor_count = floor_count_minus_one + 1;

    for _ in 0..floor_count {
        // Floor type 1
        write(1u16, &mut bytes, 16);

        let floor_partitions: usize;
        (i, floor_partitions) = read_write(i, &mut bytes, 5);

        // Build the class list
        let mut floor_partition_class_list = Vec::with_capacity(floor_partitions);
        let mut maximum_class = 0;
        for _ in 0..floor_partitions {
            let floor_partition_class: u8;
            (i, floor_partition_class) = read_write(i, &mut bytes, 4);

            floor_partition_class_list.push(floor_partition_class);
            maximum_class = maximum_class.max(floor_partition_class);
        }

        let floor_class_dimensions_list = (0..=maximum_class)
            .map(|_| {
                let class_dimensions_minus_one: u8;
                (i, class_dimensions_minus_one) = read_write(i, &mut bytes, 3);

                let class_subclasses: u8;
                (i, class_subclasses) = read_write(i, &mut bytes, 2);

                if class_subclasses != 0 {
                    let masterbook: u8;
                    (i, masterbook) = read_write(i, &mut bytes, 8);

                    if masterbook as u16 >= codebook_count {
                        // TODO: throw proper error
                        panic!("invalid floor 1 masterbook");
                    }
                }

                for _ in 0..(1 << class_subclasses as u32) {
                    let subclass_book_plus_one: u8;
                    (i, subclass_book_plus_one) = read_write(i, &mut bytes, 8);

                    let subclass_book = subclass_book_plus_one as i16 - 1;
                    if subclass_book >= 0 && subclass_book >= codebook_count as i16 {
                        // TODO: throw proper error
                        panic!("invalid floor 1 subclass book");
                    }
                }

                class_dimensions_minus_one + 1
            })
            .collect::<Vec<_>>();

        let _floor_multiplier_minus_one: u8;
        (i, _floor_multiplier_minus_one) = read_write(i, &mut bytes, 2);

        let range_bits: usize;
        (i, range_bits) = read_write(i, &mut bytes, 4);

        floor_partition_class_list
            .into_iter()
            .for_each(|current_class_number| {
                for _ in 0..floor_class_dimensions_list[current_class_number as usize] {
                    let _x: u16;
                    (i, _x) = read_write(i, &mut bytes, range_bits);
                }
            });
    }

    // Residues
    let (mut i, residue_count_minus_one): (_, u8) = read_write(i, &mut bytes, 6);
    let residue_count = residue_count_minus_one + 1;

    for _ in 0..residue_count {
        let residue_type: u16;
        (i, residue_type) = read(i, 2);
        write(residue_type, &mut bytes, 16);

        if residue_type > 2 {
            return Err(WemError::Corrupt("invalid residue type".to_string()));
        }

        let _residue_begin: u32;
        (i, _residue_begin) = read_write(i, &mut bytes, 24);

        let _residue_end: u32;
        (i, _residue_end) = read_write(i, &mut bytes, 24);

        let _residue_partition_size_minus_one: u32;
        (i, _residue_partition_size_minus_one) = read_write(i, &mut bytes, 24);

        let residue_classifications_minus_one: u8;
        (i, residue_classifications_minus_one) = read_write(i, &mut bytes, 6);
        let residue_classifications = residue_classifications_minus_one + 1;

        let residue_classbook: u8;
        (i, residue_classbook) = read_write(i, &mut bytes, 8);

        if residue_classbook as u16 >= codebook_count {
            return Err(WemError::Corrupt("residue classbook".to_string()));
        }

        let residue_cascade = (0..residue_classifications)
            .map(|_| {
                let low_bits: u8;
                (i, low_bits) = read_write(i, &mut bytes, 3);

                let bit_flag;
                (i, bit_flag) = read_bool(i);
                bytes.push(bit_flag);
                let high_bits = if bit_flag {
                    let high_bits: u8;
                    (i, high_bits) = read_write(i, &mut bytes, 5);

                    high_bits
                } else {
                    0
                };

                high_bits as u32 * 8 + low_bits as u32
            })
            .collect::<Vec<_>>();

        residue_cascade
            .into_iter()
            .try_for_each(|residue_cascade| {
                for k in 0..8 {
                    if (residue_cascade & (1 << k)) > 0 {
                        let residue_book: u8;
                        (i, residue_book) = read_write(i, &mut bytes, 8);

                        if residue_book as u16 >= codebook_count {
                            return Err(WemError::Corrupt("residue book".to_string()));
                        }
                    }
                }

                Ok(())
            })?;
    }

    // Mapping
    let (mut i, mapping_count_minus_one): (_, u8) = read_write(i, &mut bytes, 6);
    let mapping_count = mapping_count_minus_one + 1;

    for _ in 0..mapping_count {
        // Mapping type 0
        write(0u16, &mut bytes, 16);

        let submaps_flag;
        (i, submaps_flag) = read_write_bool(i, &mut bytes);
        let submaps = if submaps_flag {
            let submaps_minus_one: u8;
            (i, submaps_minus_one) = read_write(i, &mut bytes, 4);

            submaps_minus_one + 1
        } else {
            1
        };

        let square_polar_flag;
        (i, square_polar_flag) = read_write_bool(i, &mut bytes);
        if square_polar_flag {
            let coupling_steps_minus_one: u16;
            (i, coupling_steps_minus_one) = read_write(i, &mut bytes, 8);
            let coupling_steps = coupling_steps_minus_one + 1;

            for _ in 0..coupling_steps {
                let magnitude: u32;
                (i, magnitude) = read_write(i, &mut bytes, log2(fmt.channels as u32 - 1) as usize);

                let angle: u32;
                (i, angle) = read_write(i, &mut bytes, log2(fmt.channels as u32 - 1) as usize);

                if angle == magnitude
                    || magnitude >= fmt.channels as u32
                    || angle >= fmt.channels as u32
                {
                    return Err(WemError::Corrupt("coupling".to_string()));
                }
            }
        }

        let mapping_reserved: u8;
        (i, mapping_reserved) = read_write(i, &mut bytes, 2);
        if mapping_reserved != 0 {
            return Err(WemError::Corrupt(
                "mapping reserved field nonzero".to_string(),
            ));
        }

        if submaps > 1 {
            for _ in 0..fmt.channels {
                let mapping_mux: u8;
                (i, mapping_mux) = read_write(i, &mut bytes, 4);

                if mapping_mux >= submaps {
                    return Err(WemError::Corrupt("mapping mux >= submaps".to_string()));
                }
            }
        }

        for _ in 0..submaps {
            let _time_config: u8;
            (i, _time_config) = read_write(i, &mut bytes, 8);

            let floor_number: u8;
            (i, floor_number) = read_write(i, &mut bytes, 8);
            if floor_number >= floor_count {
                return Err(WemError::Corrupt("floor mapping".to_string()));
            }

            let residue_number: u8;
            (i, residue_number) = read_write(i, &mut bytes, 8);
            if residue_number >= residue_count {
                return Err(WemError::Corrupt("residue mapping".to_string()));
            }
        }
    }

    // Mode count
    let (mut i, mode_count_minus_one): (_, u8) = read_write(i, &mut bytes, 6);
    let mode_count = mode_count_minus_one + 1;

    let mode_blockflag = (0..mode_count)
        .map(|_| {
            let block_flag;
            (i, block_flag) = read_write_bool(i, &mut bytes);

            // Window type
            write(0u16, &mut bytes, 16);
            // Transform type
            write(0u16, &mut bytes, 16);

            let mapping: u8;
            (i, mapping) = read_write(i, &mut bytes, 8);
            if mapping >= mapping_count {
                Err(WemError::Corrupt("invalid mode mapping".to_string()))
            } else {
                Ok(block_flag)
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let mode_bits = utils::log2(mode_count_minus_one as u32);

    // Framing
    write(1u8, &mut bytes, 1);

    // TODO: verify size
    Ok((bytes.into_vec(), mode_blockflag, mode_bits))
}
