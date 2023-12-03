mod error;
pub mod level;
pub mod manifest;
pub mod note;
pub mod song;
mod song_xml;
pub mod xblock;

use manifest::Manifest;
use psarc::{ArchiveReadError, PlaystationArchive};
use rodio_wem::WemDecoder;
use song::Song;
use song_xml::XmlSong;

use crate::{
    error::{Result, RocksmithArchiveError},
    xblock::{SimplifiedEntity, Xblock},
};

/// Parsed Rockmith 2014 .psarc file.
#[derive(Debug, Clone)]
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
        if entities.is_empty() {
            return Err(RocksmithArchiveError::MissingData(
                "xblock entities".to_string(),
            ));
        }

        // TODO: place this in a more logical place, with async loading
        let manifests = entities
            .iter()
            .filter_map(|entity| {
                entity
                    .manifest
                    .as_ref()
                    .map(|manifest_path| Manifest::parse(&archive, manifest_path))
            })
            .collect::<Result<Vec<_>>>()?;

        // Get the song bank
        let bnk_bytes = read_urn_file(
            &archive,
            &entities[0]
                .sound_bank
                .as_ref()
                .ok_or_else(|| RocksmithArchiveError::MissingData("bnk file".to_string()))?,
            "bnk",
        )?;

        // Get the wem filename from the bnk file
        let wem_filenames = bnk::wem_filenames(&bnk_bytes)?;
        if wem_filenames.is_empty() {
            return Err(RocksmithArchiveError::MissingData("bnk".to_string()));
        }

        // Construct the full path
        let song_path = archive.try_path_ending_with(&wem_filenames[0])?.to_string();

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
    pub fn wem(&self) -> Result<Vec<u8>> {
        Ok(self.archive.read_file_with_path(self.song_path())?)
    }

    /// Get the bytes from the music embedded with the chosen song and recode it to a proper vorbis
    /// decoder.
    pub fn music_decoder(&self) -> Result<WemDecoder> {
        Ok(WemDecoder::new(&self.wem()?)?)
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

    /// Get the parsed song information for a section.
    pub fn parse_song_info(&self, section_index: usize) -> Result<Song> {
        let asset = &self.entities[section_index]
            .sng_asset
            .as_ref()
            .ok_or_else(|| RocksmithArchiveError::MissingData("sng file".to_string()))?;

        // Get the song XML
        let xml_string = read_urn_file_string(&self.archive, asset, "xml")?;

        let xml = XmlSong::parse(&xml_string)?;

        Ok(Song::from(xml))
    }
}

/// Read a file as bytes from an urn file.
fn read_urn_file(archive: &PlaystationArchive, urn: &str, extension: &str) -> Result<Vec<u8>> {
    let urn_filename = urn_filename(urn)?;

    // Get the path of the file
    let archive_path = archive.try_path_ending_with(&format!("{}.{}", urn_filename, extension))?;

    Ok(archive.read_file_with_path(archive_path)?)
}

/// Read a file as a string from an urn file.
fn read_urn_file_string(
    archive: &PlaystationArchive,
    urn: &str,
    extension: &str,
) -> Result<String> {
    let urn_filename = urn_filename(urn)?;

    // Get the path of the file
    let archive_path = archive.try_path_ending_with(&format!("{}.{}", urn_filename, extension))?;

    // TODO: clean up archive api
    let index = archive.index_for_path(archive_path).unwrap();

    Ok(archive.read_file_as_string(index)?)
}

/// Get the filename from an urn path.
fn urn_filename(urn: &str) -> Result<&str> {
    urn.split(":")
        .last()
        .ok_or_else(|| RocksmithArchiveError::InvalidUrnPath(urn.to_string()))
}
