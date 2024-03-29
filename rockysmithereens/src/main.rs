mod asset;
mod filesystem;
mod note;
mod note_view;
mod player;
mod preview;
mod ui;
mod wem;

use std::{path::PathBuf, sync::Mutex};

#[cfg(feature = "profile")]
use bevy_puffin::PuffinTracePlugin;

use asset::{RocksmithAsset, RocksmithAssetLoader};
use bevy::{
    asset::AssetPlugin,
    log::LogPlugin,
    prelude::{AddAsset, App, AssetServer, Assets, Handle, Res, ResMut},
    DefaultPlugins,
};
use bevy_egui::EguiPlugin;
use clap::Parser;
use filesystem::FilesystemPlugin;
use note::NotePlugin;
use note_view::NoteViewPlugin;
use player::PlayerPlugin;

use preview::PreviewPlugin;
use rockysmithereens_parser::SongFile;
use ui::UiPlugin;
use wem::WemPlugin;

/// Command line arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, propagate_version = true)]
struct Cli {
    /// Path to a Rocksmith '*.psarc' file.
    #[clap(value_parser)]
    path: Option<PathBuf>,
}

/// Which phase of the game we are in.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Phase {
    /// No song has been chosen yet.
    SongSelectionMenu,
    /// A song has been selected but no arrangement yet.
    ArrangementSelectionMenu,
    /// We are parsing and loading the required files.
    Loading,
    /// A song will be playing now.
    Playing,
}

/// Game state.
#[derive(Default)]
pub struct State {
    /// Song asset.
    handle: Handle<RocksmithAsset>,
    /// Which song got selected.
    current_song: Option<usize>,
    /// The current difficulty.
    difficulty: usize,
}

// TODO: figure out how to make this a resource, the current problem is that AssetIo doesn't accept
// any state
lazy_static::lazy_static! {
    /// The song state.
    pub static ref LOADED_SONG: Mutex<Option<SongFile>> = Mutex::new(None);
}

fn main() {
    let mut app = App::new();

    // Profiling
    #[cfg(feature = "profile")]
    {
        app = app.add_plugin(PuffinTracePlugin::new());
    }

    app.add_plugins_with(DefaultPlugins, |group| {
        // Disable the logging because we'll be using puffin
        group
            .disable::<LogPlugin>()
            // Insert the custom filesystem asset plugin at the right position
            .add_before::<AssetPlugin, _>(FilesystemPlugin)
    })
    .add_plugin(EguiPlugin)
    .add_plugin(WemPlugin)
    .add_plugin(PlayerPlugin)
    .add_plugin(UiPlugin)
    .add_plugin(PreviewPlugin)
    .add_plugin(NoteViewPlugin)
    .add_plugin(NotePlugin)
    .add_state(Phase::SongSelectionMenu)
    .init_resource::<State>()
    .add_asset::<RocksmithAsset>()
    .init_asset_loader::<RocksmithAssetLoader>()
    .add_startup_system(cli_setup)
    .add_system(song_loader)
    .run();
}

/// Handle CLI arguments.
fn cli_setup(
    mut state: ResMut<State>,
    asset_server: Res<AssetServer>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
) {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load the asset if set
    if let Some(path) = cli.path {
        state.handle = asset_server.load::<RocksmithAsset, _>(&*path);

        phase.set(Phase::ArrangementSelectionMenu).unwrap();
    }
}

/// Event listener for switching to the song virtual filesystem.
pub fn song_loader(state: ResMut<State>, mut rocksmith_assets: ResMut<Assets<RocksmithAsset>>) {
    // Move the asset to this filesystem
    let asset = rocksmith_assets.remove(&state.handle);
    if let Some(file) = asset {
        *LOADED_SONG.lock().unwrap() = Some(file.0);
    }
}
