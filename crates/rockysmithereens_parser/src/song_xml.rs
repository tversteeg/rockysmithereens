use serde::Deserialize;

use crate::error::{Result, RocksmithArchiveError};

/// Parsed song information.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongXml {
    version: String,
    title: String,
    levels: Levels,
}
impl SongXml {
    /// Parse the XML string into this object.
    pub fn parse(xml: &str) -> Result<Self> {
        Ok(quick_xml::de::from_str(xml)?)
    }

    /// Get the highest difficulty.
    pub fn highest_difficulty(&self) -> Option<u8> {
        self.levels
            .levels
            .iter()
            .map(|level| level.difficulty)
            .max()
    }

    /// Find the level matching the difficulty.
    pub fn into_level_with_difficulty(self, difficulty: u8) -> Result<Level> {
        self.levels
            .levels
            .into_iter()
            .find(|level| level.difficulty == difficulty)
            .ok_or_else(|| RocksmithArchiveError::NoLevelWithDifficulty(difficulty))
    }

    /// Get all levels as an iterator.
    pub fn levels_iter(&self) -> impl Iterator<Item = &Level> {
        self.levels.levels.iter()
    }
}

/// Newtype for levels with different difficulties.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Levels {
    #[serde(rename = "level")]
    levels: Vec<Level>,
    /// Should match with the length of the levels.
    count: usize,
}

/// Information for a single level.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    /// Difficulty rating of this level.
    difficulty: u8,
    /// Camera positions.
    anchors: Anchors,
    /// Notes.
    notes: Notes,
    /// Chords.
    chords: Chords,
}

impl Level {
    /// Get all notes between the timerange.
    pub fn notes_between_time_iter(
        &self,
        start_time: f32,
        end_time: f32,
    ) -> impl Iterator<Item = &Note> {
        self.notes
            .notes
            .iter()
            .filter(move |note| note.time >= start_time && note.time < end_time)
    }

    /// Get all chord notes between the timerange.
    pub fn chord_notes_between_time_iter(
        &self,
        start_time: f32,
        end_time: f32,
    ) -> impl Iterator<Item = &ChordNote> {
        self.chords
            .chords
            .iter()
            .filter(move |chord| chord.time >= start_time && chord.time < end_time)
            .map(move |chord| chord.notes.iter())
            .flatten()
    }
}

/// Where the camera should be.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchors {
    #[serde(rename = "anchor")]
    anchors: Vec<Anchor>,
    count: usize,
}

/// Single camera position in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    /// When the camera should be placed at the location.
    time: f32,
    /// At which fret the camera should zoom in.
    fret: u8,
    /// How much the camera should be zoomed in.
    width: f32,
}

/// All the notes for this section.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notes {
    #[serde(rename = "note", default)]
    notes: Vec<Note>,
    count: usize,
}

/// A singe note in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// When the note should be struck.
    pub time: f32,
    /// Which fret to play this note on.
    pub fret: i8,
    /// Which string to play this note on.
    pub string: i8,
    /// Whether it should bend.
    ///
    /// Means the bend values array will be filled.
    bend: Option<u8>,
    /// The values when `bend == 1`.
    bend_values: Option<BendValues>,
    /*
    /// Whether this note should be played with the left hand.
    left_hand: i8,
    /// Whether this note should be played with the right hand.
    right_hand: i8,
    /// Which direction the string should be picked.
    pick_direction: i8,
    // TODO: find out what it does
    link_next: i8,
    // TODO: find out what it does
    slide_unpitch_to: i8,
    // TODO: find out what it does
    slide_to: i8,
    /// Whether this note is a hammer-on.
    hammer_on: i8,
    /// Whether this note is a harmonic note.
    harmonic: i8,
    /// Whether this note should be muted.
    mute: i8,
    /// Whether this note should be muted with the palm.
    palm_mute: i8,
    /// Whether this note should be plucked.
    pluck: i8,
    /// Whether this note should be pulled off.
    pull_off: i8,
    /// How hard this should be slapped.
    slap: i8,
    /// Whether this should be tapped.
    tap: i8,
    /// Whether this is a vibrato note.
    vibrato: i8,
    /// How much this note needs to be sustained.
    sustain: f32,
    // TODO: find out what it does
    harmonic_pinch: i8,
    // TODO: find out what it does
    hopo: i8,
    // TODO: find out what it does
    ignore: i8,
    */
}

/// All the bend values for this note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BendValues {
    #[serde(rename = "bendValue")]
    bend_values: Vec<BendValue>,
}

/// A singe bend_value in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BendValue {
    /// When the bend part should be struck.
    time: f32,
    // TODO: find out what it does
    step: f32,
    // TODO: find out what it does
    unk5: Option<i32>,
}

/// All the chords for this section.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chords {
    #[serde(rename = "chord", default)]
    chords: Vec<Chord>,
    count: usize,
}

/// A single chord in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chord {
    /// When the chord will be struck.
    time: f32,
    /// Which cord it is.
    ///
    /// The name and other information can be found with this ID.
    chord_id: i16,
    /// Notes for this chord.
    #[serde(rename = "chordNote", default)]
    notes: Vec<ChordNote>,
}

/// A single note for a chord in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordNote {
    /// When the note should be struck.
    pub time: f32,
    /// Which fret to play this note on.
    pub fret: i8,
    /// Which string to play this note on.
    pub string: i8,
}
