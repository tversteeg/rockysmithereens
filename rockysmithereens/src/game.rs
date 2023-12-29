use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use itertools::Itertools;
use miette::{IntoDiagnostic, Result};
use rockysmithereens_parser::SongFile;
use rodio::{OutputStream, Sink};
use vek::Vec2;

/// Position of the line from the left side.
const START_X: f32 = 20.0;

/// Note game entity.
pub struct Note {
    /// Time the note will be triggered in seconds.
    pub trigger_time_secs: f32,
    /// Which string the note is on.
    pub string: u8,
    /// Which fret the note is on.
    pub fret: u8,
}

impl Note {
    /// Draw a note on the screen.
    pub fn render(&self, elapsed_secs: f32) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        let pos = Vec2::new(
            START_X + (self.trigger_time_secs - elapsed_secs) * 300.0,
            50.0 + self.string as f32 * 30.0 + self.fret as f32,
        );
        //font.render(&format!("{}", self.fret), pos.as_(), canvas);
    }
}

/// Main game.
pub struct Game {
    /// Playing song.
    song: SongFile,
    /// Audio sink.
    sink: Sink,
    /// Audio stream.
    stream: OutputStream,
    /// Position of the player.
    elapsed: Arc<RwLock<(Duration, Instant)>>,
    /// Position of the player in seconds.
    elapsed_secs: f32,
    /// How long the song will play.
    total_duration: Duration,
    /// All notes, grouped by the second.
    notes: HashMap<u32, Vec<Note>>,
}

impl Game {
    /// Start the game with a song.
    pub fn new(song: SongFile, current_song: usize) -> Result<Self> {
        // Decode the song
        let decoder = song.music_decoder().into_diagnostic()?;

        // How long the song will play
        let total_duration = decoder.total_duration().into_diagnostic()?;

        // Get a reference to how long the player has been playing
        let elapsed = decoder.elapsed_ref();

        // Play the song
        let (stream, stream_handle) = OutputStream::try_default().into_diagnostic()?;
        let sink = Sink::try_new(&stream_handle).into_diagnostic()?;
        sink.append(decoder);

        // Use the current time as the snapshot
        let elapsed_secs = 0.0;

        // Parse the notes
        let notes = song
            .parse_song_info(current_song)
            .map_err(|err| miette::miette!("Error parsing song: {err:?}"))?
            .notes_iter()
            // Group by time
            .map(|note| {
                (
                    note.time.floor() as u32,
                    Note {
                        trigger_time_secs: note.time,
                        string: note.string,
                        fret: note.fret,
                    },
                )
            })
            .into_group_map();

        Ok(Self {
            song,
            sink,
            stream,
            elapsed,
            elapsed_secs,
            total_duration,
            notes,
        })
    }

    /// Update step of the game.
    pub fn update(&mut self) {
        // Calculate the actual elapsed time from the moment the snapshot is taken and the duration
        let elapsed = {
            let (elapsed, snapshot) = *self.elapsed.read().unwrap();
            elapsed + (Instant::now() - snapshot)
        };
        self.elapsed_secs = elapsed.as_secs_f32();
    }

    /// Render the game.
    pub fn render(&mut self) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        // Take the next couple of seconds so we don't have to loop through all notes
        for time in (self.elapsed_secs as u32)..(self.elapsed_secs as u32 + 3) {
            if let Some(notes) = self.notes.get(&time) {
                // Render all notes in the time bucket
                for note in notes.iter() {
                    note.render(self.elapsed_secs);
                }
            }
        }
    }
}
