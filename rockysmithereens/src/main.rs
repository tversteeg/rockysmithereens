mod asset;
mod filesystem;
mod player;
mod ui;
mod wem;

use std::{path::PathBuf, sync::Mutex};

use asset::{RocksmithAsset, RocksmithAssetLoader};
use bevy::{
    asset::AssetPlugin,
    prelude::{AddAsset, App, AssetServer, Assets, Handle, Res, ResMut},
    DefaultPlugins,
};
use bevy_egui::{EguiPlugin};
use clap::Parser;
use filesystem::FilesystemPlugin;
use player::PlayerPlugin;

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
}

lazy_static::lazy_static! {
/// The song state.
    pub static ref LOADED_SONG: Mutex<Option<SongFile>> = Mutex::new(None);
}

fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| {
            // Insert the custom filesystem asset plugin at the right position
            group.add_before::<AssetPlugin, _>(FilesystemPlugin)
        })
        .add_plugin(EguiPlugin)
        .add_plugin(WemPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(UiPlugin)
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
