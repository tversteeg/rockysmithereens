use crate::{level::Level, note::Note, song_xml::XmlSong};

/// The whole song with the different levels.
#[derive(Debug, Clone)]
pub struct Song {
    /// All the levels.
    pub levels: Vec<Level>,
}

impl Song {
    /// Get all notes for a certain difficulty between two timestamps.
    pub fn notes_between_time_iter(
        &self,
        start_time: f32,
        end_time: f32,
        difficulty: u8,
    ) -> impl Iterator<Item = &Note> {
        self.levels
            .iter()
            .filter(move |level| level.difficulty <= difficulty)
            .flat_map(move |level| level.notes_between_time_iter(start_time, end_time))
    }

    /// Get all notes.
    pub fn notes_iter(&self) -> impl Iterator<Item = &Note> {
        self.levels.iter().flat_map(move |level| level.notes_iter())
    }
}

impl From<XmlSong> for Song {
    fn from(xml: XmlSong) -> Self {
        let levels = xml
            .into_levels_iter()
            .map(|xml_level| Level::from(xml_level))
            .collect();

        Self { levels }
    }
}
