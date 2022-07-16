use std::{ffi::OsStr};

use bevy::prelude::{AssetServer, Commands, Query, Res, ResMut};
use bevy_egui::{
    egui::{CentralPanel, ScrollArea},
    EguiContext,
};
use rfd::FileDialog;

use crate::{
    asset::RocksmithAsset,
    preview::{Preview, UnloadedPath},
    Phase, State,
};

/// The UI for selecting a song.
#[profiling::function]
pub fn ui(
    mut commands: Commands,
    mut context: ResMut<EguiContext>,
    mut state: ResMut<State>,
    asset_server: Res<AssetServer>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
    previews: Query<&Preview>,
) {
    CentralPanel::default().show(context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Open a Rocksmith '*.psarc' file");

                    // Open the file when the button is clicked
                    if ui.button("Open file..").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Rocksmith", &["psarc"])
                            .pick_file()
                        {
                            state.handle = asset_server.load::<RocksmithAsset, _>(path);
                            phase.set(Phase::ArrangementSelectionMenu).unwrap();
                        }
                    }
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Open a folder containing Rocksmith '*.psarc' files");

                    // Load a quick preview from all files in the folder
                    if ui.button("Open folder..").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Rocksmith", &["psarc"])
                            .pick_folder()
                        {
                            // TODO: use proper error handling
                            // Read all files in the folder
                            let mut files = std::fs::read_dir(path)
                                .unwrap()
                                .collect::<Result<Vec<_>, _>>()
                                .unwrap();

                            // Sort them alphabetically
                            files.sort_by_key(|file| file.path());

                            // Create a component for each psarc file
                            for path in files {
                                let path = path.path();
                                if path.extension() == Some(OsStr::new("psarc")) {
                                    commands.spawn().insert(UnloadedPath(path));
                                }
                            }
                        }
                    }
                });
            });
        });

        // List the different songs
        ScrollArea::vertical().show(ui, |ui| {
            for preview in previews.iter() {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&preview.song);
                        ui.label("-");
                        ui.label(&preview.artist);
                        ui.label("-");
                        ui.label(&preview.album);
                    });

                    if ui
                        .button(preview.path.file_name().unwrap().to_str().unwrap())
                        .clicked()
                    {
                        let path_str = preview.path.to_str().unwrap();
                        state.handle = asset_server.load::<RocksmithAsset, _>(path_str);
                        phase.set(Phase::ArrangementSelectionMenu).unwrap();
                    }
                });
            }
        });
    });
}
