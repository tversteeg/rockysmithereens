use std::collections::HashMap;

use psarc::{ArchiveReadError, PlaystationArchive};
use serde::Deserialize;
use serde_json::Value;

use crate::error::Result;

/// The JSON manifest with the song information.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    insert_root: String,
    model_name: String,
    iteration_version: u16,
    entries: HashMap<String, Entry>,
}

impl Manifest {
    pub fn parse(archive: &PlaystationArchive, path: &str) -> Result<Self> {
        // Read the file from the archive
        let json = archive.read_rs_file_as_string(path, "json")?;

        Ok(serde_json::from_str(&json)?)
    }

    /// Get the attributes, takes a lot of shortcuts.
    pub fn attributes(&'_ self) -> &'_ Attributes {
        &self
            .entries
            .iter()
            .next()
            .expect("no attributes")
            .1
            .attributes
    }
}

/// Various data entries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Entry {
    attributes: Attributes,
}

/// Various data entries.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attributes {
    pub arrangement_properties: HashMap<String, u8>,
    pub arrangement_sort: u8,
    pub arrangement_type: u8,
    pub arrangement_name: String,
    pub full_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub song_name: String,
    pub song_length: f32,
}
