mod asset;
mod event;
mod filesystem;
mod player;
mod wem;

use std::path::PathBuf;

use asset::{RocksmithAsset, RocksmithAssetLoader};
use bevy::{
    asset::AssetPlugin,
    audio::AudioOutput,
    math::Vec3,
    pbr::{PbrBundle, PointLight, PointLightBundle, StandardMaterial},
    prelude::{
        shape, AddAsset, App, AssetServer, Assets, Color, Commands, EventWriter, Handle, Mesh,
        PerspectiveCameraBundle, Res, ResMut, Transform,
    },
    DefaultPlugins,
};
use bevy_egui::{
    egui::{CentralPanel, Window},
    EguiContext, EguiPlugin,
};
use clap::{Parser, Subcommand};
use event::{LoadedEvent, StartEvent};
use filesystem::FilesystemPlugin;
use rfd::FileDialog;

use rockysmithereens_parser::SongFile;
use rodio_wem::WemDecoder;
use wem::WemPlugin;

/// Command line arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, propagate_version = true)]
struct Cli {
    /// Path to a Rocksmith '*.psarc' file.
    #[clap(value_parser)]
    path: Option<PathBuf>,
}

/// Game state.
#[derive(Debug, Default)]
pub struct State {
    /// Song asset.
    handle: Handle<RocksmithAsset>,
    /// Which song got selected.
    current_song: Option<usize>,
}

fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| {
            // Insert the custom filesystem asset plugin at the right position
            group.add_before::<AssetPlugin, _>(FilesystemPlugin)
        })
        .add_plugin(EguiPlugin)
        .add_plugin(WemPlugin)
        .init_resource::<State>()
        .add_asset::<RocksmithAsset>()
        .init_asset_loader::<RocksmithAssetLoader>()
        .add_event::<StartEvent>()
        .add_event::<LoadedEvent>()
        .add_startup_system(setup)
        .add_system(ui)
        .add_system(player::loaded_listener)
        .run();
}

/// set up a simple 3D scene
fn setup(mut state: ResMut<State>, asset_server: Res<AssetServer>) {
    // Parse command line arguments
    let cli = Cli::parse();
    // Load the asset if set
    if let Some(path) = cli.path {
        state.handle = asset_server.load::<RocksmithAsset, _>(&*path);
    }
}

/// The UI for selecting a song.
fn ui(
    mut context: ResMut<EguiContext>,
    mut state: ResMut<State>,
    asset_server: Res<AssetServer>,
    rocksmith_assets: ResMut<Assets<RocksmithAsset>>,
    mut start_events: EventWriter<StartEvent>,
) {
    // Don't draw the selection UI when a song has already been selected
    if state.current_song.is_some() {
        return;
    }

    CentralPanel::default().show(context.ctx_mut(), |ui| {
        ui.label("Open a Rocksmith '*.psarc' file");

        // Open the file when the button is clicked
        if ui.button("Open file..").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("Rocksmith", &["psarc"])
                .pick_file()
            {
                state.handle = asset_server.load::<RocksmithAsset, _>(path);
            }
        }

        let asset = rocksmith_assets.get(&state.handle);
        // A song has been loaded
        if let Some(file) = asset {
            // List the different songs
            for (i, manifests) in file.0.manifests.iter().enumerate() {
                ui.group(|ui| {
                    let attributes = manifests.attributes();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&attributes.song_name);
                        ui.label("-");
                        ui.label(&attributes.artist_name);
                        ui.label("-");
                        ui.label(&attributes.album_name);
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(&attributes.arrangement_name);
                        ui.label(&format!(
                            "{} min {} sec",
                            (attributes.song_length / 60.0).ceil(),
                            (attributes.song_length % 60.0).ceil()
                        ));
                    });

                    if ui.button("Choose song").clicked() {
                        state.current_song = Some(i);
                        start_events.send_default();
                    }
                });
            }
        }
    });
}
