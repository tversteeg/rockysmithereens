use crate::player::{MusicController, NOTE_SPAWN_TIME};
use bevy::prelude::{Res, ResMut};
use bevy_egui::{
    egui::{
        plot::{Line, Plot, Text, VLine, Value, Values},
        Color32, TextStyle, TopBottomPanel,
    },
    EguiContext,
};
use rockysmithereens_parser::song::Song;

/// Until how many seconds after playing the notes should be shown.
const NOTE_KEEP_PLAYING_TIME: f32 = 3.0;
/// How much bends will curve.
const BEND_FACTOR: f32 = 0.2;

/// Show the notes.
#[profiling::function]
pub fn ui(
    mut context: ResMut<EguiContext>,
    music_controller: Res<MusicController>,
    song: Res<Song>,
) {
    let time_playing_secs = music_controller.time_playing.as_secs_f32();

    // Get the notes that will be played soon
    let notes = song.notes_between_time_iter(
        time_playing_secs - NOTE_KEEP_PLAYING_TIME,
        time_playing_secs + NOTE_SPAWN_TIME,
        200,
    );

    TopBottomPanel::bottom("notes").show(context.ctx_mut(), |ui| {
        ui.style_mut().override_text_style = Some(TextStyle::Heading);

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
                    // Get the starting position of the note
                    let x = note.time - time_playing_secs;
                    let pos = Value::new(x, note.string);
                    let color = string_number_to_color(note.string);

                    // Draw a line when a sustain is played
                    if let Some(sustain) = note.sustain {
                        // Add the bend if applicable
                        let (first_pos, second_pos) = match note.bend {
                            Some((start_bend, end_bend)) => (
                                Value::new(x, note.string as f32 + start_bend * BEND_FACTOR),
                                Value::new(
                                    x + sustain,
                                    note.string as f32 + end_bend * BEND_FACTOR,
                                ),
                            ),
                            None => (pos, Value::new(x + sustain, note.string as f32)),
                        };

                        plot_ui.line(
                            Line::new(Values::from_values(vec![first_pos, second_pos]))
                                .color(color)
                                .width(1.0),
                        );
                    }

                    // Draw an X when it's a mute
                    if note.mute {
                        plot_ui.text(Text::new(pos, "X").color(Color32::GRAY));
                    }

                    // Draw the number for the note if it's not an intermediate note
                    if note.bend.is_none() {
                        if note.slide_to_next {
                            // Draw slides differently
                            plot_ui.text(Text::new(pos, format!("{}>", note.fret)).color(color));
                        } else {
                            plot_ui.text(Text::new(pos, format!("{}", note.fret)).color(color));
                        }
                    }
                });

                // A line at zero
                plot_ui.vline(VLine::new(0.0));
            });
    });
}

/// Get the string color as an egui color.
pub fn string_number_to_color(string: u8) -> Color32 {
    match string {
        // E
        0 => Color32::from_rgb(0xFF, 0x00, 0x00),
        // A
        1 => Color32::from_rgb(0xFF, 0xFF, 0x00),
        // D
        2 => Color32::from_rgb(0x99, 0x99, 0xFF),
        // G
        3 => Color32::from_rgb(0xFF, 0x99, 0x00),
        // B
        4 => Color32::from_rgb(0x00, 0xFF, 0x00),
        // e
        5 => Color32::from_rgb(0xFF, 0x00, 0xFF),
        _ => Color32::from_rgb(0xFF, 0xFF, 0xFF),
    }
}
