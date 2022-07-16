use crate::{note::Note, song_xml::XmlLevel};

/// Information about the level of a song.
#[derive(Debug, Default, Clone)]
pub struct Level {
    /// All the notes.
    pub notes: Vec<Note>,
    /// The difficulty of this level.
    pub difficulty: u8,
}

impl Level {
    /// Get all notes between the timerange.
    pub fn notes_between_time_iter(
        &self,
        start_time: f32,
        end_time: f32,
    ) -> impl Iterator<Item = &Note> {
        self.notes
            .iter()
            .filter(move |note| note.time >= start_time && note.time < end_time)
    }
}

impl From<XmlLevel> for Level {
    fn from(xml: XmlLevel) -> Self {
        let difficulty = xml.difficulty;

        // Combine all regular with chord notes and convert them to our type
        let (regular_notes_iter, chords_iter) = xml.into_iters();
        let notes = regular_notes_iter
            .map(|note| Vec::<Note>::from(note).into_iter())
            .chain(chords_iter.map(|xml| Vec::<Note>::from(xml).into_iter()))
            .flatten()
            .collect();

        Level { notes, difficulty }
    }
}
