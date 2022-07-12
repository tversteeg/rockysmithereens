use std::{ops::Deref, time::Duration};

use bevy::{
    audio::{Audio, AudioSink},
    core::Time,
    input::Input,
    prelude::{
        App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, ResMut, SystemSet,
    },
};
use bevy_egui::{
    egui::{
        plot::{Plot, Points, Text, VLine, Value, Values},
        CentralPanel, Color32, TopBottomPanel, Vec2,
    },
    EguiContext,
};
use rockysmithereens_parser::song_xml::SongXml;

use crate::{
    player::{Level, MusicController, NOTE_SPAWN_TIME},
    wem::WemSource,
    Phase, State, LOADED_SONG,
};

/// Show the notes.
#[profiling::function]
pub fn ui(
    mut context: ResMut<EguiContext>,
    music_controller: Res<MusicController>,
    //level: Res<Level>,
    xml: Res<SongXml>,
) {
    let time_playing_secs = music_controller.time_playing.as_secs_f32();

    // Get the notes that will be played soon
    let notes = xml
        .levels_iter()
        .map(|level| {
            level.notes_between_time_iter(time_playing_secs, time_playing_secs + NOTE_SPAWN_TIME)
        })
        .flatten();

    // Get the chord notes that will be played soon
    let chord_notes = xml
        .levels_iter()
        .map(|level| {
            level.chord_notes_between_time_iter(
                time_playing_secs,
                time_playing_secs + NOTE_SPAWN_TIME,
            )
        })
        .flatten();

    TopBottomPanel::bottom("notes").show(context.ctx_mut(), |ui| {
        ui.label("Notes");

        Plot::new("notes_plot")
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .include_x(-1.0)
            .include_x(NOTE_SPAWN_TIME)
            .include_y(-1.0)
            .include_y(7.0)
            .show_x(false)
            .show_y(false)
            .show_axes([false, true])
            .height(300.0)
            .show(ui, |plot_ui| {
                // Each regular note
                notes.for_each(|note| {
                    plot_ui.text(
                        Text::new(
                            Value::new(note.time - time_playing_secs, note.string),
                            format!("{}", note.fret),
                        )
                        .color(match note.string {
                            // E
                            0 => Color32::from_rgb(0xFF, 0x00, 0x00),
                            // A
                            1 => Color32::from_rgb(0xFF, 0xFF, 0x00),
                            // D
                            2 => Color32::from_rgb(0x44, 0x44, 0xFF),
                            // G
                            3 => Color32::from_rgb(0xFF, 0x99, 0x00),
                            // B
                            4 => Color32::from_rgb(0x00, 0xFF, 0x00),
                            // e
                            5 => Color32::from_rgb(0xFF, 0x00, 0xFF),
                            _ => Color32::from_rgb(0xFF, 0xFF, 0xFF),
                        }),
                    )
                });

                // Each chord note
                chord_notes.for_each(|note| {
                    plot_ui.text(
                        Text::new(
                            Value::new(note.time - time_playing_secs, note.string),
                            format!("{}", note.fret),
                        )
                        .color(match note.string {
                            // E
                            0 => Color32::from_rgb(0xFF, 0x00, 0x00),
                            // A
                            1 => Color32::from_rgb(0xFF, 0xFF, 0x00),
                            // D
                            2 => Color32::from_rgb(0x44, 0x44, 0xFF),
                            // G
                            3 => Color32::from_rgb(0xFF, 0x99, 0x00),
                            // B
                            4 => Color32::from_rgb(0x00, 0xFF, 0x00),
                            // e
                            5 => Color32::from_rgb(0xFF, 0x00, 0xFF),
                            _ => Color32::from_rgb(0xFF, 0xFF, 0xFF),
                        }),
                    )
                });

                // A line at zero
                plot_ui.vline(VLine::new(0.0));
            });
    });
}
