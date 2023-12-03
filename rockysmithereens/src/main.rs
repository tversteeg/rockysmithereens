mod game;
mod ui;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

use game::Game;
use miette::Result;
use pixel_game_lib::{
    gui::{
        button::{Button, ButtonRef},
        label::{Label, LabelRef},
        Gui, GuiBuilder, Widget,
    },
    vek::{Extent2, Vec2},
    window::{KeyCode, WindowConfig},
};
use rfd::AsyncFileDialog;
use rockysmithereens_parser::SongFile;
use taffy::{prelude::Size, style::Style};
use ui::home::Homescreen;

/// Which screen we are currently on.
pub enum Phase {
    /// Gui for the homescreen.
    Homescreen(Homescreen),
    /// Gui for playing.
    Game(Game),
}

/// Game state passed around the update and render functions.
pub struct State {
    /// Current screen.
    screen: Phase,
    /// Bytes for the file.
    loaded_song: Arc<RwLock<Option<SongFile>>>,
}

/// Open an empty window.
#[tokio::main]
async fn main() -> Result<()> {
    // Window configuration with default pixel size and scaling
    let window_config = WindowConfig {
        buffer_size: Extent2::new(600, 480),
        ..Default::default()
    };

    // The file to open, can be set from all threads
    let loaded_song = Arc::new(RwLock::new(None));

    // Create the shareable game state
    let state = State {
        screen: Phase::Homescreen(Homescreen::new(window_config.buffer_size.as_())),
        loaded_song: loaded_song.clone(),
    };

    // Open a new thread waiting for the file dialog to be activated
    let (open_file_tx, open_file_rx) = tokio::sync::watch::channel(());
    tokio::spawn(async move {
        // Repeat this task for ever
        let mut open_file_rx = open_file_rx.clone();
        loop {
            // Wait for the value to "change"
            open_file_rx.changed().await.expect("Sender dropped");

            // Open the file dialog
            if let Some(file) = AsyncFileDialog::new()
                .add_filter("Rocksmith", &["psarc"])
                .pick_file()
                .await
            {
                // Read the bytes from the file
                let bytes = file.read().await;

                // Parse the bytes into the song
                // TODO: report error
                let song = SongFile::parse(&bytes).expect("Failed parsing song");

                // Set the value so the state can read it
                *loaded_song.write().unwrap() = Some(song);

                println!("Loaded and parsed song '{}'", file.file_name());
            }
        }
    });

    // Open the window and start the game-loop
    pixel_game_lib::window(
        state,
        window_config.clone(),
        // Update loop exposing input events we can handle, this is where you would handle the game logic
        move |state, input, mouse_pos, _dt| {
            match &mut state.screen {
                Phase::Homescreen(homescreen) => {
                    if homescreen.update(input, mouse_pos) {
                        // Release the lock for the blocking async thread
                        open_file_tx.send(()).expect("Readers dropped");
                    }

                    // Switch the screen when a song is loaded
                    if let Some(song) = state.loaded_song.read().unwrap().as_ref() {
                        state.screen =
                            Phase::Game(Game::new(song.clone()).expect("Failed loading song"));
                    }
                }
                Phase::Game(game) => {
                    game.update(input, mouse_pos);
                }
            }

            // Exit when escape is pressed
            input.key_pressed(KeyCode::Escape)
        },
        // Render loop exposing the pixel buffer we can mutate
        move |state, canvas, _dt| match &mut state.screen {
            Phase::Homescreen(homescreen) => {
                homescreen.render(canvas);
            }
            Phase::Game(game) => {
                game.render(canvas);
            }
        },
    )?;

    Ok(())
}
