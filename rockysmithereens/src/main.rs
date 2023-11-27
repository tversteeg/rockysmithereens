mod ui;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

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
use taffy::{prelude::Size, style::Style};
use ui::home::Homescreen;

/// Game state passed around the update and render functions.
pub struct State {
    /// Gui for the homescreen.
    homescreen: Homescreen,
}

/// Open an empty window.
#[tokio::main]
async fn main() -> Result<()> {
    // Window configuration with default pixel size and scaling
    let window_config = WindowConfig {
        buffer_size: Extent2::new(600, 480),
        ..Default::default()
    };

    // Create the shareable game state
    let state = State {
        homescreen: Homescreen::new(window_config.buffer_size.as_()),
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
            {}
        }
    });

    // Open the window and start the game-loop
    pixel_game_lib::window(
        state,
        window_config.clone(),
        // Update loop exposing input events we can handle, this is where you would handle the game logic
        move |state, input, mouse_pos, _dt| {
            if state.homescreen.update(input, mouse_pos) {
                // Release the lock for the blocking async thread
                open_file_tx.send(()).expect("Readers dropped");
            }

            // Exit when escape is pressed
            input.key_pressed(KeyCode::Escape)
        },
        // Render loop exposing the pixel buffer we can mutate
        move |state, canvas, _dt| {
            state.homescreen.render(canvas);
        },
    )?;

    Ok(())
}
