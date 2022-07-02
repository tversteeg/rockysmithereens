use bitvec::{order::Lsb0, prelude::BitVec, view::BitView};
use nom::{error::context, number::complete::le_u16};

use crate::{
    error::Result,
    utils::{read, read_write, write},
};

/// A vorbis data packet.
#[derive(Debug)]
pub struct Packet {
    /// Raw data for the packet.
    pub data: Vec<u8>,
}

impl Packet {
    pub fn parse<'a>(
        data: &'a [u8],
        mode_blockflag: &[bool],
        mode_bits: usize,
        previous_mode_blockflag: bool,
    ) -> Result<(&'a [u8], (Self, bool))> {
        let (packet, size) = context("packet size", le_u16)(data)?;
        let size = size as usize;

        // Change the mod flags
        let mut bits = BitVec::<u8, Lsb0>::new();

        // Packet type is audio
        write(0u8, &mut bits, 1);

        // Get mode number from first byte
        let i = packet.view_bits();
        let (i, mode_number): (_, u8) = read_write(i, &mut bits, mode_bits);

        // Read the rest of the input bits
        let (_, remainder): (_, u8) = read(i, 8 - mode_bits);

        let current_mode_blockflag = mode_blockflag[mode_number as usize];
        let packet_offset = if current_mode_blockflag {
            // Long window, look at next frame
            let next_block = &packet[size..];
            let next_block_flag = if next_block.is_empty() {
                false
            } else {
                let (next_block, next_block_size) =
                    context("next packet size", le_u16)(next_block)?;
                if next_block_size > 0 {
                    let (_, next_mode_number): (_, u8) = read(next_block.view_bits(), mode_bits);

                    mode_blockflag[next_mode_number as usize]
                } else {
                    false
                }
            };

            // Previouws window type bit
            bits.push(previous_mode_blockflag);

            // Next window type bit
            bits.push(next_block_flag);

            // Push remainder
            write(remainder, &mut bits, 8 - mode_bits);

            1
        } else {
            0
        };

        // Copy the rest of the buffer
        bits.extend(&packet[packet_offset..size]);

        let data = bits.into_vec();

        Ok((&packet[size..], (Self { data }, current_mode_blockflag)))
    }
}

/// Parse the data bytes into packets.
pub fn parse_into_packets(
    mut i: &[u8],
    mode_blockflag: Vec<bool>,
    mode_bits: u32,
) -> Result<Vec<Packet>> {
    let mut packets = Vec::new();

    let mut previous_mode_blockflag = false;
    while !i.is_empty() {
        let packet;
        (i, (packet, previous_mode_blockflag)) = Packet::parse(
            i,
            &mode_blockflag,
            mode_bits as usize,
            previous_mode_blockflag,
        )?;
        packets.push(packet);
    }

    Ok(packets)
}
