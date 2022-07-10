use bevy::prelude::{AssetServer, Res, ResMut};
use bevy_egui::{egui::CentralPanel, EguiContext};
use rfd::FileDialog;

use crate::{asset::RocksmithAsset, Phase, State};

/// The UI for selecting a song.
#[profiling::function]
pub fn ui(
    mut context: ResMut<EguiContext>,
    mut state: ResMut<State>,
    asset_server: Res<AssetServer>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
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
                phase.set(Phase::ArrangementSelectionMenu).unwrap();
            }
        }
    });
}
