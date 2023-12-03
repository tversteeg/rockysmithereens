use std::collections::HashMap;

use psarc::PlaystationArchive;
use serde::Deserialize;

use crate::error::Result;

/// The JSON manifest with the song information.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    pub insert_root: String,
    pub model_name: String,
    pub iteration_version: u16,
    entries: HashMap<String, Entry>,
}

impl Manifest {
    pub fn parse(archive: &PlaystationArchive, path: &str) -> Result<Self> {
        // Read the file from the archive
        let json = archive.read_rs_file_as_string(path, "json")?;

        let manifest = serde_json::from_str(&json)?;

        // TODO: Remove the vocal bit, we don't care about it

        Ok(manifest)
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Entry {
    attributes: Attributes,
}

/// Various data entries.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Attributes {
    /// Different properties of the song.
    pub arrangement_properties: ArrangementProperties,
    pub chords: HashMap<String, HashMap<String, Vec<u32>>>,
    pub chord_templates: Vec<ChordTemplate>,
    pub dynamic_visual_density: Vec<f32>,
    pub tuning: Tuning,
    pub phrase_iterations: Vec<PhraseIteration>,
    pub phrases: Vec<Phrase>,
    pub sections: Vec<Section>,
    pub techniques: HashMap<String, HashMap<String, Vec<u32>>>,
    pub arrangement_sort: u8,
    pub arrangement_type: u8,
    pub arrangement_name: String,
    pub full_name: String,
    pub artist_name: String,
    pub artist_name_sort: String,
    pub album_name: String,
    pub album_name_sort: String,
    pub song_name: String,
    pub song_name_sort: String,
    pub song_year: u16,
    pub song_length: f32,
    pub song_average_tempo: f32,
    pub song_offset: f32,
    pub song_partition: i32,
    pub song_volume: f32,
    pub preview_volume: f32,
    pub song_key: String,
    pub last_conversion_date_time: String,
    #[serde(rename = "MasterID_PS3")]
    pub master_id_ps3: i32,
    #[serde(rename = "MasterID_XBox360")]
    pub master_id_xbox360: i32,
    pub max_phrase_difficulty: u8,
    pub relative_difficulty: u8,
    pub target_score: i32,
    #[serde(rename = "Score_MaxNotes")]
    pub score_max_notes: f32,
    #[serde(rename = "Score_PNV")]
    pub score_pnv: f32,
    pub capo_fret: f32,
    pub cent_offset: f32,
    pub dlc: bool,
    pub dlc_key: String,
    pub dna_chords: f32,
    pub dna_riffs: f32,
    pub dna_solo: f32,
    pub easy_mastery: f32,
    pub medium_mastery: f32,
    pub notes_easy: f32,
    pub notes_medium: f32,
    pub notes_hard: f32,
    pub song_diff_easy: f32,
    pub song_diff_med: f32,
    pub song_diff_hard: f32,
    pub song_difficulty: f32,
    pub leaderboard_challenge_rating: i32,
    #[serde(rename = "MasterID_RDV")]
    pub master_id_rdv: i32,
    #[serde(rename = "PersistentID")]
    pub persistent_id: String,
    pub shipping: bool,
    pub sku: String,
    pub tone_a: String,
    pub tone_b: String,
    pub tone_base: String,
    pub tone_c: String,
    pub block_asset: String,
    pub preview_bank_path: String,
    #[serde(rename = "ShowlightsXML")]
    pub show_lights_xml: String,
    pub song_asset: String,
    pub song_bank: String,
    pub song_event: String,
    pub song_xml: String,
    pub album_art: String,
    pub manifest_urn: String,
}

impl Attributes {
    /// Get the name of the song.
    pub fn name(&self) -> &str {
        if self.song_name.is_empty() {
            &self.song_name_sort
        } else {
            &self.song_name
        }
    }

    /// Get the artist of the song.
    pub fn artist(&self) -> &str {
        if self.artist_name.is_empty() {
            &self.artist_name_sort
        } else {
            &self.artist_name
        }
    }

    /// Get the album of the song.
    pub fn album(&self) -> &str {
        if self.album_name.is_empty() {
            &self.album_name_sort
        } else {
            &self.album_name
        }
    }
}

/// Template for a chord.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChordTemplate {
    pub chord_id: u32,
    pub chord_name: String,
    pub fingers: Vec<i32>,
    pub frets: Vec<i32>,
}

// TODO: use bool instead of u8
/// Properties of this arrangement.
///
/// The number is a boolean where 0 is false and 1 zero.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArrangementProperties {
    pub bonus_arr: u8,
    #[serde(rename = "Metronome")]
    pub metronome: u8,
    pub path_lead: u8,
    pub path_rhythm: u8,
    pub path_bass: u8,
    pub route_mask: u8,
    pub represent: u8,
    pub standard_tuning: u8,
    pub non_standard_chords: u8,
    pub barre_chords: u8,
    pub power_chords: u8,
    pub drop_d_power: u8,
    pub finger_picking: u8,
    pub pick_direction: u8,
    pub double_stops: u8,
    pub palm_mutes: u8,
    pub harmonics: u8,
    pub pinch_harmonics: u8,
    pub hopo: u8,
    pub tremolo: u8,
    pub slides: u8,
    pub unpitched_slides: u8,
    pub bends: u8,
    pub tapping: u8,
    pub vibrato: u8,
    pub fret_hand_mutes: u8,
    pub slap_pop: u8,
    pub two_finger_picking: u8,
    pub fifths_and_octaves: u8,
    pub syncopation: u8,
    pub bass_pick: u8,
    pub sustain: u8,
}

/// Different string tunings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Tuning {
    pub string_0: i8,
    pub string_1: i8,
    pub string_2: i8,
    pub string_3: i8,
    pub string_4: i8,
    pub string_5: i8,
}

/// Information about the different phrases.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PhraseIteration {
    pub phrase_index: u16,
    pub max_difficulty: u8,
    pub name: String,
    pub start_time: f64,
    pub end_time: f64,
}

/// Seems to be double information for [`PhraseIteration`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Phrase {
    pub max_difficulty: u16,
    pub name: String,
    pub iteration_count: u8,
}

/// Information about the different sections.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Section {
    pub name: String,
    #[serde(rename = "UIName")]
    pub ui_name: String,
    pub number: u8,
    pub start_time: f32,
    pub end_time: f32,
    pub start_phrase_iteration_index: u8,
    pub end_phrase_iteration_index: u8,
    pub is_solo: bool,
}
