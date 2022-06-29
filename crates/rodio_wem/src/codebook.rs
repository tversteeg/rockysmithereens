use bitvec::{field::BitField, order::Lsb0, prelude::BitVec, view::BitView};
use nom::{error::context, number::complete::le_u32};

use crate::{
    error::{Result, WemError},
    utils::{read, read_bool},
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
    pub fn rebuild(&self, codebook_index: usize) -> Result<(usize, Vec<u8>)> {
        // Get the codebook data belonging to the index
        let codebook_data = self.codebook(codebook_index)?;

        // Convert the raw bytes to a bitvec so we can read individual bits
        let i = codebook_data.view_bits();

        // Write everything to a new buffer
        let mut out = BitVec::<_, Lsb0>::new();

        // Identifier
        out.extend(&0x564342u32.view_bits::<Lsb0>()[..24]);

        // Read the metadata from the codebook
        let (i, dimensions): (_, u32) = read(i, 4);
        out.extend(&dimensions.view_bits::<Lsb0>()[..16]);

        let (i, entry_count): (_, u32) = read(i, 14);
        out.extend(&entry_count.view_bits::<Lsb0>()[..24]);

        // Ordered flag
        let (mut i, ordered) = read_bool(i);
        if ordered {
            todo!();
        } else {
            // Codewords
            let codeword_lengths_length: u8;
            (i, codeword_lengths_length) = read(i, 3);

            let sparse;
            (i, sparse) = read_bool(i);
            out.push(sparse);

            for _ in 0..entry_count {
                // Read and write the present bool if sparse is set
                let present = if sparse {
                    let present;
                    (i, present) = read_bool(i);
                    out.push(present);

                    present
                } else {
                    true
                };

                if present {
                    let codeword_length: u8;
                    (i, codeword_length) = read(i, codeword_lengths_length as usize);
                    out.extend(&codeword_length.view_bits::<Lsb0>()[..5]);
                }
            }
        }

        // Lookup table
        let (mut i, lookup_type): (_, u8) = read(i, 1);
        out.extend(&lookup_type.view_bits::<Lsb0>()[..4]);

        if lookup_type == 1 {
            let min: u32;
            (i, min) = read(i, 32);
            out.extend(&min.view_bits::<Lsb0>()[..32]);

            let max: u32;
            (i, max) = read(i, 32);
            out.extend(&max.view_bits::<Lsb0>()[..32]);

            let sequence_flag;
            (i, sequence_flag) = read_bool(i);
            out.push(sequence_flag);
        } else if lookup_type != 0 {
            return Err(WemError::Corrupt("lookup type".to_string()));
        }

        let bits_read = codebook_data.view_bits::<Lsb0>().len() - i.len();

        Ok((bits_read / 8, out.into_vec()))
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
