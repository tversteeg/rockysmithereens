mod error;
mod manifest;
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
    pub archive: PlaystationArchive,
}

impl SongFile {
    /// Parse the Rocksmith archive file.
    pub fn parse(file: &[u8]) -> Result<Self> {
        // Parse the playstation archive file
        let archive = PlaystationArchive::parse(file)?;
        dbg!(archive.paths_iter().collect::<Vec<_>>());

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

        Ok(Self {
            manifests,
            entities,
            archive,
        })
    }

    /// Read a file from the archive.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        Ok(self.archive.read_file_with_path(path)?)
    }

    /// Get the bytes from the music embedded with the chosen song.
    pub fn wem(&self, index: usize) -> Result<Vec<u8>> {
        let _path = self.entities[index]
            .sng_asset
            .as_ref()
            .ok_or_else(|| RocksmithArchiveError::MissingData("wem".into()))?;

        Ok(self.archive.read_rs_file("", "wem")?)
    }

    /// Get the bytes from the music embedded with the chosen song and recode it to a proper vorbis
    /// decoder.
    pub fn vorbis(&self, index: usize) -> Result<WemDecoder> {
        Ok(WemDecoder::new(&self.wem(index)?)?)
    }

    /// Path for the album art file.
    pub fn album_art_path(&self) -> Result<String> {
        Ok("gfxassets/album_art/album_witchcraftsong_256.dds".into())
    }
}
