mod error;
pub mod manifest;
mod xblock;

use manifest::Manifest;
use psarc::PlaystationArchive;
use rodio_wem::WemDecoder;

use crate::{
    error::{Result, RocksmithArchiveError},
    xblock::{SimplifiedEntity, Xblock},
};

/// Parsed Rockmith 2014 .psarc file.
#[derive(Debug)]
pub struct SongFile {
    pub entities: Vec<SimplifiedEntity>,
    pub manifests: Vec<Manifest>,
    /// Archive containing all the files.
    pub archive: PlaystationArchive,
    /// The path to the song file.
    song_path: String,
}

impl SongFile {
    /// Parse the Rocksmith archive file.
    pub fn parse(file: &[u8]) -> Result<Self> {
        // Parse the playstation archive file
        let archive = PlaystationArchive::parse(file)?;

        // Get the xblock file
        let xblock_indices = archive
            .enumerated_file_paths_by_extension_iter(".xblock")
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if xblock_indices.is_empty() {
            return Err(RocksmithArchiveError::NotARocksmitheFile);
        }

        // TODO: handle multiple block files
        let xblock = Xblock::parse(&archive.read_file_as_string(xblock_indices[0])?)?;

        // Get the required song properties
        let entities = xblock.simplified_entities_iter().collect::<Vec<_>>();

        // TODO: place this in a more logical place, with async loading
        let manifests = entities
            .iter()
            .filter_map(|entity| {
                entity
                    .manifest
                    .as_ref()
                    .map(|manifest_path| Manifest::parse(&archive, manifest_path))
            })
            .collect::<Result<_>>()?;

        // Find the song path
        let urn_path = self.entities[0]
            .sound_bank
            .as_ref()
            .expect("No sound bank file");

        // Get the filename from the urn path
        let urn_filename = urn_path.split(":").last().expect("Invalid URN path");

        // Get the path of the bnk file
        let bnk_path = self
            .archive
            .path_ending_with(&format!("{}.bnk", urn_filename))
            .expect("No song file in psarc");

        // Get the wem filename from the bnk file
        let wem_filenames = bnk::wem_filenames(&self.archive.read_file_with_path(bnk_path)?)?;

        // Construct the full path
        let song_path = self.archive.path_ending_with(wem_filenames[0]);

        Ok(Self {
            manifests,
            entities,
            archive,
            song_path,
        })
    }

    /// Read a file from the archive.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        Ok(self.archive.read_file_with_path(path)?)
    }

    /// Get the bytes from the music embedded with the chosen song.
    pub fn wem(&self, index: usize) -> Result<Vec<u8>> {
        Ok(self.archive.read_file_with_path(self.song_path())?)
    }

    /// Get the bytes from the music embedded with the chosen song and recode it to a proper vorbis
    /// decoder.
    pub fn vorbis(&self, index: usize) -> Result<WemDecoder> {
        Ok(WemDecoder::new(&self.wem(index)?)?)
    }

    /// Path for the album art file.
    pub fn album_art_path(&self) -> Option<&str> {
        self.archive
            .path_ending_with("256.dds")
            .or_else(|| self.archive.path_ending_with("128.dds"))
            .or_else(|| self.archive.path_ending_with("64.dds"))
    }

    /// Path for the vorbis wem file.
    pub fn song_path(&self) -> &str {
        &self.song_path
    }
}
