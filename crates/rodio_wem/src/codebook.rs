use bitvec::{order::Lsb0, prelude::BitVec, view::BitView};
use nom::{error::context, number::complete::le_u32};

use crate::{
    error::{Result, WemError},
    utils::{log2, read, read_bool, read_write, read_write_bool, write},
};

/// Represents an collection of external codebooks loaded from a file.
#[derive(Debug)]
pub struct CodebookLibrary<'a> {
    /// The offsets in the data for each codebook.
    offsets: Vec<u32>,
    /// The raw data for all codebooks.
    data: &'a [u8],
}

impl<'a> CodebookLibrary<'a> {
    /// Create the library from a binary.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        // Get the last 4 bytes of the stream as the offset for the list of offsets
        let (_, offsets_offset) =
            context("codebook offsets offset", le_u32)(&bytes[(bytes.len() - 4)..])?;

        let codebook_count = (bytes.len() - offsets_offset as usize) / 4;
        let data = &bytes[..offsets_offset as usize];

        // Get all offsets from the remaining data
        let offsets =
            (0..codebook_count)
                .map(|i| {
                    Ok(context("codebook offset", le_u32)(
                        &bytes[(offsets_offset as usize + i * 4)..],
                    )?
                    .1)
                })
                .collect::<Result<Vec<_>>>()?;

        Ok(Self { offsets, data })
    }

    /// Rebuild the vorbis header input codebook.
    pub fn rebuild(&self, codebook_index: usize) -> Result<BitVec<u8, Lsb0>> {
        // Get the codebook data belonging to the index
        let codebook_data = self.codebook(codebook_index)?;

        // Convert the raw bytes to a bitvec so we can read individual bits
        let i = codebook_data.view_bits();

        // Write everything to a new buffer
        let mut out = BitVec::<_, Lsb0>::new();

        // Identifier
        write(0x564342u32, &mut out, 24);

        // Read the metadata from the codebook
        let (i, dimensions): (_, u32) = read(i, 4);
        write(dimensions, &mut out, 16);

        let (i, entry_count): (_, u32) = read(i, 14);
        write(entry_count, &mut out, 24);

        // Ordered flag
        let (mut i, ordered) = read_write_bool(i, &mut out);
        if ordered {
            let _initial_length: u8;
            (i, _initial_length) = read_write(i, &mut out, 5);

            let mut current_entry = 0;
            while current_entry < entry_count {
                let number: u32;
                (i, number) = read_write(
                    i,
                    &mut out,
                    crate::utils::log2(entry_count - current_entry) as usize,
                );

                current_entry += number;
            }
        } else {
            // Codewords
            let codeword_lengths_length: u8;
            (i, codeword_lengths_length) = read(i, 3);

            if codeword_lengths_length == 0 || codeword_lengths_length > 5 {
                return Err(WemError::Corrupt(
                    "nonsense codeword lengths length".to_string(),
                ));
            }

            let sparse;
            (i, sparse) = read_write_bool(i, &mut out);

            for _ in 0..entry_count {
                // Read and write the present bool if sparse is set
                let present = if sparse {
                    let present;
                    (i, present) = read_write_bool(i, &mut out);

                    present
                } else {
                    true
                };

                if present {
                    let codeword_length: u8;
                    (i, codeword_length) = read(i, codeword_lengths_length as usize);
                    write(codeword_length, &mut out, 5);
                }
            }
        }

        // Lookup table
        let (mut i, lookup_type): (_, u8) = read(i, 1);
        write(lookup_type, &mut out, 4);

        if lookup_type == 1 {
            let _min: u32;
            (i, _min) = read_write(i, &mut out, 32);

            let _max: u32;
            (i, _max) = read_write(i, &mut out, 32);

            let value_length: u8;
            (i, value_length) = read_write(i, &mut out, 4);

            let sequence_flag;
            (i, sequence_flag) = read_bool(i);
            out.push(sequence_flag);

            let quantvals = CodebookLibrary::quantvals(entry_count, dimensions);
            for _ in 0..quantvals {
                let _val: u32;
                (i, _val) = read_write(i, &mut out, value_length as usize + 1);
            }
        } else if lookup_type != 0 {
            return Err(WemError::Corrupt("lookup type".to_string()));
        }

        let bits_read = codebook_data.view_bits::<Lsb0>().len() - i.len();

        // Ensure that we used all bytes
        if codebook_data.len() != bits_read / 8 + 1 {
            Err(WemError::Corrupt("codebook size mismatch".to_string()))
        } else {
            Ok(out)
        }
    }

    /// Get the data for a specific codebook.
    pub fn codebook(&self, index: usize) -> Result<&'a [u8]> {
        let first_offset = self
            .offsets
            .get(index)
            .map(|v| *v as usize)
            .ok_or_else(|| WemError::MissingData("codebook".to_string()))?;
        let last_offset = self
            .offsets
            .get(index + 1)
            .map(|v| *v as usize)
            .unwrap_or(self.data.len());

        Ok(&self.data[first_offset..last_offset])
    }

    /// Get the amount of quant values that should be parsed.
    pub fn quantvals(entries: u32, dimensions: u32) -> u32 {
        let bits = log2(entries) as u32;
        let mut vals = entries >> ((bits - 1) * (dimensions - 1) / dimensions);

        loop {
            let acc = vals.pow(dimensions);
            let acc1 = (vals + 1).pow(dimensions);

            if acc <= entries && acc1 > entries {
                return vals;
            } else if acc > entries {
                vals -= 1;
            } else {
                vals += 1;
            }
        }
    }
}

impl CodebookLibrary<'static> {
    /// Create the library from the embedded aoTuV binary.
    pub fn from_aotuv() -> Self {
        // Should never fail because the binary is included and will never be changed
        Self::from_bytes(include_bytes!("./packed_codebooks_aoTuV_603.bin")).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::CodebookLibrary;

    #[test]
    fn load_aotuv() {
        let lib = CodebookLibrary::from_aotuv();
        assert_eq!(lib.offsets.len(), 599);
        assert_eq!(lib.offsets[1], 8);
    }
}
