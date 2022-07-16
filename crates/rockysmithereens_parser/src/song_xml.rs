use serde::Deserialize;

use crate::error::{Result, RocksmithArchiveError};

/// Parsed song information.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlSong {
    version: String,
    title: String,
    levels: XmlLevels,
}
impl XmlSong {
    /// Parse the XML string into this object.
    pub fn parse(xml: &str) -> Result<Self> {
        std::fs::write("/tmp/song.xml", xml).unwrap();
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
    pub fn into_level_with_difficulty(self, difficulty: u8) -> Result<XmlLevel> {
        self.levels
            .levels
            .into_iter()
            .find(|level| level.difficulty == difficulty)
            .ok_or_else(|| RocksmithArchiveError::NoLevelWithDifficulty(difficulty))
    }

    /// Get all levels as an iterator.
    pub fn into_levels_iter(self) -> impl Iterator<Item = XmlLevel> {
        self.levels.levels.into_iter()
    }
}

/// Newtype for levels with different difficulties.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlLevels {
    #[serde(rename = "level")]
    levels: Vec<XmlLevel>,
}

/// Information for a single level.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlLevel {
    /// Difficulty rating of this level.
    pub difficulty: u8,
    /// Camera positions.
    //anchors: XmlAnchors,
    /// Notes.
    notes: XmlNotes,
    /// Chords.
    chords: XmlChords,
}

impl XmlLevel {
    /// Consume and move to iterators.
    pub(crate) fn into_iters(
        self,
    ) -> (
        impl Iterator<Item = XmlNote>,
        impl Iterator<Item = XmlChord>,
    ) {
        (self.notes.notes.into_iter(), self.chords.chords.into_iter())
    }
}

/// Where the camera should be.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlAnchors {
    #[serde(rename = "anchor")]
    anchors: Vec<XmlAnchor>,
}

/// Single camera position in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlAnchor {
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
pub struct XmlNotes {
    #[serde(rename = "note", default)]
    notes: Vec<XmlNote>,
}

/// A singe note in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlNote {
    /// When the note should be struck.
    pub time: f32,
    /// Which fret to play this note on.
    pub fret: i8,
    /// Which string to play this note on.
    pub string: i8,
    /// To which fret to slide if applicable.
    pub slide_to: Option<i8>,
    /// Whether it should bend.
    ///
    /// Means the bend values array will be filled.
    pub bend: Option<f32>,
    /// The values when `bend == 1`.
    pub bend_values: Option<XmlBendValues>,
    /// Whether this note should be muted.
    pub mute: Option<i8>,
    /// Whether this note should be muted with the palm.
    pub palm_mute: Option<i8>,
    /// How much this note needs to be sustained.
    pub sustain: Option<f32>,
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
    /// Whether this note is a hammer-on.
    hammer_on: i8,
    /// Whether this note is a harmonic note.
    harmonic: i8,
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
    // TODO: find out what it does
    harmonic_pinch: i8,
    // TODO: find out what it does
    hopo: i8,
    // TODO: find out what it does
    ignore: i8,
    */
}

impl XmlNote {
    /// Iterator over all bend values.
    pub fn bend_values_iter(&self) -> impl Iterator<Item = &XmlBendValue> {
        self.bend_values
            .iter()
            .flat_map(|bend_values| bend_values.bend_values.iter())
    }
}

/// All the bend values for this note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlBendValues {
    #[serde(rename = "bendValue")]
    bend_values: Vec<XmlBendValue>,
}

/// A singe bend_value in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlBendValue {
    /// When the bend part should be struck.
    pub time: f32,
    // TODO: find out what it does
    pub step: Option<f32>,
    // TODO: find out what it does
    unk5: Option<i32>,
}

/// All the chords for this section.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlChords {
    #[serde(rename = "chord", default)]
    chords: Vec<XmlChord>,
}

/// A single chord in time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmlChord {
    /// When the chord will be struck.
    time: f32,
    /// Which cord it is.
    ///
    /// The name and other information can be found with this ID.
    pub chord_id: i16,
    /// Notes for this chord.
    #[serde(rename = "chordNote", default)]
    pub notes: Vec<XmlNote>,
}
