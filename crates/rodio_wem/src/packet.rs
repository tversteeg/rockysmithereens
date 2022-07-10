use bitvec::{order::Lsb0, prelude::BitVec, view::BitView};

use nom::{error::context, number::complete::le_u16};

use crate::{
    error::Result,
    utils::{read, read_write, write},
};

/// A vorbis data packet.
pub struct Packet {
    /// Raw data for the packet.
    pub data: Vec<u8>,
    /// Whether the mode flag is set.
    mode_block_flag: bool,
}

impl Packet {
    #[profiling::function]
    pub fn parse<'a>(
        data: &'a [u8],
        mode_block_flags: &[bool],
        mode_bits: usize,
        previous_window_flag: bool,
    ) -> Result<(&'a [u8], Self)> {
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

        let current_mode_block_flag = mode_block_flags[mode_number as usize];
        if current_mode_block_flag {
            // Long window, look at next frame
            let next_block = &packet[size..];
            let next_window_flag = if next_block.is_empty() {
                false
            } else {
                let (next_block, next_block_size) =
                    context("next packet size", le_u16)(next_block)?;
                if next_block_size > 0 {
                    let (_, next_mode_number): (_, u8) = read(next_block.view_bits(), mode_bits);

                    mode_block_flags[next_mode_number as usize]
                } else {
                    false
                }
            };

            // Previouws window type bit
            bits.push(previous_window_flag);

            // Next window type bit
            bits.push(next_window_flag);
        }

        // Push remainder
        write(remainder, &mut bits, 8 - mode_bits);

        // Copy the rest of the buffer
        bits.extend(&packet[1..size]);

        let data = bits.into_vec();

        Ok((
            &packet[size..],
            Self {
                data,
                mode_block_flag: current_mode_block_flag,
            },
        ))
    }
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("data", &self.data.len())
            .field("mode_block_flag", &self.mode_block_flag)
            .finish()
    }
}

/// Parse the data bytes into packets.
#[profiling::function]
pub fn parse_into_packets(
    mut i: &[u8],
    mode_block_flag: Vec<bool>,
    mode_bits: u32,
) -> Result<Vec<Packet>> {
    let mut packets = Vec::new();

    // Keep track of the block_flag of the previous packet
    let mut previous_mode_block_flag = false;
    while !i.is_empty() {
        let packet;
        (i, packet) = Packet::parse(
            i,
            &mode_block_flag,
            mode_bits as usize,
            previous_mode_block_flag,
        )?;
        previous_mode_block_flag = packet.mode_block_flag;

        packets.push(packet);
    }

    Ok(packets)
}
