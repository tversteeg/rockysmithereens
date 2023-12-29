mod game;
mod ui;

use std::{
    io::Stdout,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use crossterm::{
    event::{Event, KeyCode, KeyEvent},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use game::Game;
use miette::{Context, IntoDiagnostic, Result};
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{
        block::{Position, Title},
        Block, Borders, List, ListItem, ListState, Paragraph,
    },
    Frame, Terminal,
};
use rfd::AsyncFileDialog;
use rockysmithereens_parser::SongFile;
use tokio::signal::unix::SignalKind;
use ui::{
    filetree::{FileTree, FileTreeState},
    list::StatefulList,
};

/// Application state.
pub enum App {
    MainMenu {
        /// List for the main menu items.
        main_menu_list_state: StatefulList,
    },
    SelectSong {
        /// Select the song state.
        select_song_state: FileTreeState,
    },
}

impl App {
    /// Start a new application.
    pub fn new() -> Self {
        Self::main_menu()
    }

    /// Switch to the main menu state.
    pub fn main_menu() -> Self {
        let main_menu_list_state = StatefulList::with_items(&["Open File", "Quit"]);

        Self::MainMenu {
            main_menu_list_state,
        }
    }

    /// Switch to the select song state.
    pub fn select_song() -> Result<Self> {
        let select_song_state = FileTreeState::from_current_dir()?;

        Ok(Self::SelectSong { select_song_state })
    }

    /// Render the current phase.
    pub fn render(&mut self, frame: &mut Frame) {
        // Draw the title
        let title = Block::new()
            .title(Title::from("Rockysmithereens").alignment(Alignment::Center))
            .title(Title::from(env!("CARGO_PKG_VERSION")).alignment(Alignment::Center))
            .title(
                Title::from("press 'q' to quit")
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::TOP | Borders::BOTTOM);

        // Layout inside the title block
        let main_layout = title.inner(frame.size());

        frame.render_widget(title, frame.size());

        // Render based on which phase we are in
        match self {
            App::MainMenu {
                main_menu_list_state,
            } => {
                // Draw the main menu
                let list_items = main_menu_list_state
                    .items
                    .iter()
                    .map(|item| ListItem::new(item.clone()))
                    .collect::<Vec<_>>();
                let main_menu_list = List::new(list_items)
                    .block(
                        Block::default()
                            .title("Main Menu")
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");
                frame.render_stateful_widget(
                    main_menu_list,
                    centered_rect(main_layout, 50, 50),
                    &mut main_menu_list_state.state,
                );
            }
            App::SelectSong { select_song_state } => {
                // Draw the file selection
                let song_select_file_tree = FileTree::new();
                frame.render_stateful_widget(song_select_file_tree, main_layout, select_song_state);
            }
        }
    }

    /// Handle key events.
    fn handle_key(&mut self, key: &KeyEvent) -> Result<bool> {
        match self {
            App::MainMenu {
                main_menu_list_state,
            } => match main_menu_list_state.update(&key).as_deref() {
                Some("Open File") => {
                    *self = Self::select_song()?;
                }
                Some("Quit") => return Ok(false),
                Some(other) => panic!("Unhandled menu event '{other}'"),
                None => {}
            },
            App::SelectSong { select_song_state } => {
                select_song_state.update(&key)?;
            }
        }

        Ok(true)
    }
}

/// Open an empty window.
#[tokio::main]
async fn main() -> Result<()> {
    // Start a puffin server when profiling
    #[cfg(feature = "profiling")]
    let _puffin_server =
        puffin_http::Server::new(&format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT)).unwrap();
    #[cfg(feature = "profiling")]
    puffin::set_scopes_on(true);

    // The file to open, can be set from all threads
    let loaded_song: Arc<RwLock<Option<SongFile>>> = Arc::new(RwLock::new(None));

    // Open a new thread waiting for the file dialog to be activated
    let (open_file_tx, open_file_rx) = tokio::sync::watch::channel(());
    tokio::spawn(async move {
        // Repeat this task for ever
        let mut open_file_rx = open_file_rx.clone();
        loop {
            // Wait for the value to "change"
            if open_file_rx.changed().await.is_err() {
                // Sender got dropped, meaning the application is closing
                return;
            }

            // Open the file dialog
            if let Some(file) = AsyncFileDialog::new()
                .add_filter("Rocksmith", &["psarc"])
                .pick_file()
                .await
            {
                // Read the bytes from the file
                let bytes: Vec<u8> = file.read().await;

                // Parse the bytes into the song
                // TODO: report error
                let song = SongFile::parse(&bytes).expect("Failed parsing song");

                // Set the value so the state can read it
                *loaded_song.write().unwrap() = Some(song);

                println!("Loaded and parsed song '{}'", file.file_name());
            }
        }
    });

    // Main menu selection
    let mut app = App::new();

    // Create the UI in the terminal
    let mut terminal = setup_terminal().wrap_err("Error setting up terminal")?;

    // Main UI loop
    loop {
        #[cfg(feature = "profiling")]
        puffin::GlobalProfiler::lock().new_frame();

        #[cfg(feature = "profiling")]
        puffin::profile_scope!("tick");

        // Draw a frame
        terminal
            .draw(|frame| {
                app.render(frame);
            })
            .into_diagnostic()
            .wrap_err("Error drawing frame")?;

        // Handle events
        if crossterm::event::poll(Duration::from_millis(250))
            .into_diagnostic()
            .wrap_err("event poll failed")?
        {
            if let Event::Key(key) = crossterm::event::read()
                .into_diagnostic()
                .wrap_err("event read failed")?
            {
                if key.code == KeyCode::Char('q') {
                    // Stop the loop
                    break;
                } else {
                    // Update the app state with the key
                    if !app.handle_key(&key)? {
                        break;
                    }
                }
            }
        }
    }

    // Undo all changes to the terminal
    restore_terminal(&mut terminal).wrap_err("Error restoring terminal")?;

    Ok(())
}

/// Setup a ratatui terminal.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = std::io::stdout();

    crossterm::terminal::enable_raw_mode()
        .into_diagnostic()
        .wrap_err("Error enabling raw mode")?;

    crossterm::execute!(stdout, EnterAlternateScreen)
        .into_diagnostic()
        .wrap_err("unable to enter alternate screen")?;

    // Attach a panic hook to reset the terminal on Rust panics
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        // First create a terminal for the hook, so it can be restored
        let mut terminal_for_hook =
            Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();

        // Reset the terminal to it's original state
        restore_terminal(&mut terminal_for_hook).expect("Error restoring terminal");

        // Call the original panic hook again so the panics don't get lost
        original_hook(panic);
    }));

    // Handle signals to reset the terminal
    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt()).into_diagnostic()?;
    let mut sigquit = tokio::signal::unix::signal(SignalKind::quit()).into_diagnostic()?;
    tokio::spawn(async move {
        let signal = tokio::select! {
            _ = sigint.recv() => 1,
            _ = sigquit.recv() => 2,
        };

        // First create a terminal for the hook, so it can be restored
        let mut terminal_for_hook =
            Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap();

        // Reset the terminal to it's original state
        restore_terminal(&mut terminal_for_hook).expect("Error restoring terminal");

        // Kill the process with the signal
        std::process::exit(signal);
    });

    Terminal::new(CrosstermBackend::new(stdout))
        .into_diagnostic()
        .wrap_err("Error creating terminal")
}

/// Restorte the terminal to it's original state
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()
        .into_diagnostic()
        .wrap_err("Error disabling raw mode")?;

    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .into_diagnostic()
        .wrap_err("Error switching back to main screen")?;

    terminal
        .show_cursor()
        .into_diagnostic()
        .wrap_err("Error showing cursor")
}

/// Center a layout rectangle.
///
/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.size(), 50, 50);
/// ```
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
