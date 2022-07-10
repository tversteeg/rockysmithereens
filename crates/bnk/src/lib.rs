mod error;

use std::collections::HashMap;

pub use error::BnkError;
use error::Result;
use nom::{bytes::complete::take, error::context, number::complete::le_u32};

/// Size of each file description in the didx section.
const DIDX_FILE_SIZE: usize = 12;

/// Get the references to .wem files from the .bnk file.
#[profiling::function]
pub fn wem_filenames(bytes: &[u8]) -> Result<Vec<String>> {
    // Parse the sections from the bnk file
    let section_map = sections(bytes)?;

    // Get the data index section
    let section_data = section_map
        .get("DIDX".as_bytes())
        .ok_or_else(|| BnkError::MissingSection("DIDX".to_string()))?;

    // Each file ID is packed with a set size
    let files = section_data.len() / DIDX_FILE_SIZE;
    (0..files)
        .map(|index| {
            let offset = index * DIDX_FILE_SIZE;

            let i = &section_data[offset..offset + DIDX_FILE_SIZE];
            let (_, wem_file_id) = context("bnk didx section file id", le_u32)(i)?;

            Ok(format!("{}.wem", wem_file_id))
        })
        .collect::<Result<Vec<_>>>()
}

/// Get all sections.
#[profiling::function]
pub fn sections(mut i: &[u8]) -> Result<HashMap<[u8; 4], &[u8]>> {
    let mut result: HashMap<[u8; 4], &[u8]> = HashMap::new();

    // Read all bytes
    while !i.is_empty() {
        let identifier;
        (i, identifier) = context("bnk section identifier", take(4u8))(i)?;
        let size;
        (i, size) = context("bnk section size", le_u32)(i)?;
        let data;
        (i, data) = context("bnk section data", take(size))(i)?;

        result.insert(
            identifier
                .try_into()
                .map_err(|_| BnkError::Corrupt("could not convert nom bytes".to_string()))?,
            data,
        );
    }

    Ok(result)
}
