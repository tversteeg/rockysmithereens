use bevy::prelude::{Res, ResMut};
use bevy_egui::{
    egui::{DragValue, TopBottomPanel},
    EguiContext,
};

use crate::{player::MusicController, Phase, State, LOADED_SONG};

/// The UI for selecting an arrangement for the song.
#[profiling::function]
pub fn ui(
    mut context: ResMut<EguiContext>,
    mut state: ResMut<State>,
    controller: Res<MusicController>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
) {
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

                        // Restart button
                        if ui.button("Restart").clicked() {
                            phase.set(Phase::SongSelectionMenu).unwrap();
                            return;
                        }

                        // Difficulty
                        if attributes.max_phrase_difficulty > 1 {
                            ui.label("Difficulty");
                            ui.add(
                                DragValue::new(&mut state.difficulty)
                                    .clamp_range(1..=attributes.max_phrase_difficulty as usize),
                            );
                        }
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
