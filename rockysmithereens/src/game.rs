use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use blit::{Blit, BlitBuffer, BlitOptions};
use itertools::Itertools;
use miette::{IntoDiagnostic, Result};
use pixel_game_lib::{
    canvas::Canvas,
    font::Font,
    vek::{Extent2, Vec2},
    window::Input,
};
use rockysmithereens_parser::SongFile;
use rodio::{OutputStream, Sink};

use crate::ui::playing::PlayingGui;

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
    pub fn render(&self, elapsed_secs: f32, font: &Font, canvas: &mut Canvas) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        let pos = Vec2::new(
            START_X + (self.trigger_time_secs - elapsed_secs) * 300.0,
            50.0 + self.string as f32 * 30.0 + self.fret as f32,
        );
        font.render(&format!("{}", self.fret), pos.as_(), canvas);
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
    /// In-game Gui.
    gui: PlayingGui,
    /// All notes, grouped by the second.
    notes: HashMap<u32, Vec<Note>>,
    /// Background image.
    ///
    /// Cached for quick redrawing.
    background: BlitBuffer,
}

impl Game {
    /// Start the game with a song.
    pub fn new(song: SongFile, current_song: usize, window_size: Extent2<f32>) -> Result<Self> {
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

        // Setup the Gui
        let gui = PlayingGui::new(window_size);

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

        // Setup the background
        let mut background = BlitBuffer::from_buffer(
            &vec![0xFFDDDDDD; window_size.as_().product()],
            window_size.w,
            127,
        );
        let background_size = background.size();

        // Draw the start line
        for y in 0..background_size.height {
            background.pixels_mut()[START_X as usize + (y * background_size.width) as usize] =
                0xFF000000;
        }

        Ok(Self {
            song,
            sink,
            stream,
            elapsed,
            elapsed_secs,
            total_duration,
            gui,
            notes,
            background,
        })
    }

    /// Update step of the game.
    pub fn update(&mut self, input: &Input, mouse_pos: Option<Vec2<usize>>) {
        // Calculate the actual elapsed time from the moment the snapshot is taken and the duration
        let elapsed = {
            let (elapsed, snapshot) = *self.elapsed.read().unwrap();
            elapsed + (Instant::now() - snapshot)
        };
        self.elapsed_secs = elapsed.as_secs_f32();

        // Update the gui
        self.gui
            .update(elapsed, self.total_duration, input, mouse_pos);
    }

    /// Render the game.
    pub fn render(&mut self, canvas: &mut Canvas) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        // Reset the canvas
        self.background.blit(
            canvas.raw_buffer(),
            self.background.size(),
            &BlitOptions::default(),
        );

        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("render notes");

            // Render the notes
            let font = Font::default();
            // Take the next couple of seconds so we don't have to loop through all notes
            for time in (self.elapsed_secs as u32)..(self.elapsed_secs as u32 + 3) {
                if let Some(notes) = self.notes.get(&time) {
                    // Render all notes in the time bucket
                    for note in notes.iter() {
                        note.render(self.elapsed_secs, &font, canvas);
                    }
                }
            }
        }

        // Render the gui
        self.gui.render(canvas);
    }
}
