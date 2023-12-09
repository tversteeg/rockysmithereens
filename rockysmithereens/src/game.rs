use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use miette::{IntoDiagnostic, Result};
use pixel_game_lib::{
    canvas::Canvas,
    vek::{Extent2, Vec2},
    window::Input,
};
use rockysmithereens_parser::SongFile;
use rodio::{OutputStream, Sink};
use rodio_wem::WemDecoder;

use crate::ui::playing::PlayingGui;

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
    /// How long the song will play.
    total_duration: Duration,
    /// In-game Gui.
    gui: PlayingGui,
}

impl Game {
    /// Start the game with a song.
    pub fn new(song: SongFile, window_size: Extent2<f32>) -> Result<Self> {
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
        let elapsed_snapshot = Instant::now();
        let elapsed_previous = Duration::default();

        Ok(Self {
            song,
            sink,
            stream,
            elapsed,
            total_duration,
            gui,
        })
    }

    /// Update step of the game.
    pub fn update(&mut self, input: &Input, mouse_pos: Option<Vec2<usize>>) {
        // Calculate the actual elapsed time from the moment the snapshot is taken and the duration
        let elapsed = {
            let (elapsed, snapshot) = *self.elapsed.read().unwrap();
            elapsed + (Instant::now() - snapshot)
        };

        self.gui
            .update(elapsed, self.total_duration, input, mouse_pos);
    }

    /// Render the game.
    pub fn render(&mut self, canvas: &mut Canvas) {
        // Reset the canvas
        canvas.fill(0xFFFFFFFF);

        self.gui.render(canvas);
    }
}
