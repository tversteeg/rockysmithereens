mod error;

use error::Result;

/// Get the references to .wem files from the .bnk file.
pub fn wem_filenames(bytes: &[u8]) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    Ok(ids)
}
