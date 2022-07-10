use bevy::prelude::{Res, ResMut};
use bevy_egui::{egui::TopBottomPanel, EguiContext};

use crate::{player::MusicController, State, LOADED_SONG};

/// The UI for selecting an arrangement for the song.
#[profiling::function]
pub fn ui(mut context: ResMut<EguiContext>, state: Res<State>, controller: Res<MusicController>) {
    if let Some(current_song) = state.current_song {
        if let Some(song) = &*LOADED_SONG.lock().unwrap() {
            // A song has been loaded
            TopBottomPanel::top("topbar").show(context.ctx_mut(), |ui| {
                // Get the first manifest for the song information
                if let Some(manifest) = song.manifests.get(0) {
                    ui.horizontal(|ui| {
                        let attributes = manifest.attributes();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(&attributes.song_name);
                            ui.label("-");
                            ui.label(&attributes.artist_name);
                            ui.label("-");
                            ui.label(&attributes.album_name);
                        });

                        ui.label(&format!(
                            "{} min {} sec",
                            (attributes.song_length / 60.0).ceil(),
                            (attributes.song_length % 60.0).ceil()
                        ));
                    });
                }

                // Show the progress of the current song
                let attributes = song.manifests[current_song].attributes();

                // Show the phrases
                super::phrases_plot::ui(ui, attributes, Some(controller.time_playing()));
            });
        }
    }
}
