use crate::song_xml::{XmlChord, XmlNote};

/// A single tone, can be part of a chord or a bend.
#[derive(Debug, Clone)]
pub struct Note {
    /// When the note should be struck.
    pub time: f32,
    /// Which fret to play this note on.
    ///
    /// `0` means it's an open string.
    pub fret: u8,
    /// Which string to play this note on.
    pub string: u8,
    /// How much to bend this (if at all).
    ///
    /// The first value is the starting position of the bend, and the second the ending position.
    pub bend: Option<(f32, f32)>,
    /// To which fret to slide if applicable.
    ///
    /// Can only be used in combination with sustain.
    pub slide_to_next: bool,
    /// Whether this note should be muted.
    ///
    /// Also includes palm mutes.
    pub mute: bool,
    /// Whether this note is part of a chord, and if yes what chord it is.
    pub chord: Option<u8>,
    /// How long this note should be held.
    pub sustain: Option<f32>,
}

impl Note {
    /// Construct a new basic note.
    pub fn new(time: f32, fret: i8, string: i8) -> Self {
        Self {
            time,
            fret: fret.max(0) as u8,
            string: string.max(0) as u8,
            mute: false,
            bend: None,
            slide_to_next: false,
            chord: None,
            sustain: None,
        }
    }
}

impl From<XmlNote> for Vec<Note> {
    fn from(xml: XmlNote) -> Self {
        let mut first = Note::new(xml.time, xml.fret, xml.string);

        // We don't differentiate between 'mute' and 'palm mute'
        first.mute = (xml.mute != Some(0) && xml.mute != None)
            || (xml.palm_mute != Some(0) && xml.palm_mute != None);

        if xml.sustain > Some(0.0) {
            first.sustain = xml.sustain;
        }

        first.slide_to_next = xml.slide_to > Some(0);

        // The first one is always a note
        std::iter::once(first.clone())
            // After that come the optional bend values
            .chain(
                xml.bend_values_iter()
                    // Keep track of the previous bend value so every note has a range
                    .scan(0f32, |previous_value, bend_value| {
                        let current_value = bend_value.step.unwrap_or(0.0);

                        let note = Some(Note {
                            time: bend_value.time,
                            bend: Some((*previous_value, current_value)),
                            ..first
                        });

                        *previous_value = current_value;

                        note
                    }),
            )
            .collect()
    }
}

impl From<XmlChord> for Vec<Note> {
    fn from(xml: XmlChord) -> Self {
        xml.notes
            .into_iter()
            .map(|chord_note| {
                Vec::<Note>::from(chord_note).into_iter().map(|mut note| {
                    note.chord = Some(xml.chord_id as u8);

                    note
                })
            })
            .flatten()
            .collect()
    }
}
