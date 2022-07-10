use bevy::prelude::{AssetServer, Assets, Handle, Image as BevyImage, Local, Res, ResMut};
use bevy_egui::{
    egui::{CentralPanel, Image, ScrollArea, TextureId},
    EguiContext,
};

use crate::{Phase, State, LOADED_SONG};

/// The UI for selecting an arrangement for the song.
#[profiling::function]
pub fn ui(
    asset_server: Res<AssetServer>,
    mut context: ResMut<EguiContext>,
    mut state: ResMut<State>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
    mut album_art_image_handle: Local<Handle<BevyImage>>,
    mut album_art_texture: Local<Option<TextureId>>,
    assets: ResMut<Assets<BevyImage>>,
) {
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        if assets.get(album_art_image_handle.clone_weak()).is_none() {
            // Load the album art
            if let Some(path) = song.album_art_path() {
                *album_art_image_handle = asset_server.load(path);
                *album_art_texture = Some(context.add_image(album_art_image_handle.clone_weak()));
            }
        }

        // A song has been loaded
        CentralPanel::default().show(context.ctx_mut(), |ui| {
            // Get the first manifest for the song information
            if let Some(manifest) = song.manifests.get(0) {
                ui.horizontal(|ui| {
                    // Show the album art if loaded
                    if let Some(album_art_texture) = *album_art_texture {
                        ui.add(Image::new(album_art_texture, [128.0, 128.0]));
                    }

                    ui.vertical(|ui| {
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
                });
            }

            // List the different songs
            ScrollArea::vertical().show(ui, |ui| {
                for (i, manifest) in song.manifests.iter().enumerate() {
                    ui.group(|ui| {
                        let attributes = manifest.attributes();

                        if ui.button(&attributes.arrangement_name).clicked() {
                            state.current_song = Some(i);

                            phase.set(Phase::Playing).unwrap();
                        }

                        // Show the phrases
                        super::phrases_plot::ui(ui, attributes, None);
                    });
                }
            });
        });
    }
}
