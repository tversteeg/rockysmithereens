use std::time::Duration;

use bevy_egui::egui::{
    plot::{Bar, BarChart, Legend, Plot, VLine},
    Color32, Ui, Vec2,
};
use rockysmithereens_parser::manifest::Attributes;

/// Draw a plot with the total song difficulties.
#[profiling::function]
pub fn ui(ui: &mut Ui, attributes: &Attributes, time_playing: Option<Duration>) {
    // Draw a line with the difficulties
    let bars = attributes
        .phrase_iterations
        .iter()
        .map(|phrase_iteration| {
            let width = phrase_iteration.end_time - phrase_iteration.start_time;
            Bar::new(
                // Offset the bar because it's centered otherwise
                phrase_iteration.start_time + width / 2.0,
                phrase_iteration.max_difficulty as f64 + 4.0,
            )
            .width(width)
            // Go more towards red based on difficulty
            .fill(Color32::from_rgb(
                248,
                252u8.saturating_sub(phrase_iteration.max_difficulty * 8),
                168u8.saturating_sub(phrase_iteration.max_difficulty * 8),
            ))
            // Offset so we can read the time signatures
            .base_offset(10.0)
        })
        .collect();
    let barchart = BarChart::new(bars).name("Difficulty");

    let plot = Plot::new(&attributes.full_name);
    plot.allow_zoom(false)
        .allow_boxed_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .show_x(false)
        .show_y(false)
        .show_axes([true, false])
        .show_background(false)
        .height(70.0)
        .legend(Legend::default().position(bevy_egui::egui::plot::Corner::LeftTop))
        // Always scale properly
        .include_y(30.0)
        // Always show the numbers
        .include_y(0.0)
        .include_x(0.0)
        .set_margin_fraction(Vec2::new(0.0, 0.0))
        .x_axis_formatter(|x, _| format!("{}m {}s", (x / 60.0).ceil(), (x % 60.0)))
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(barchart);

            // Show a vertical line with the current playing position
            if let Some(time_playing) = time_playing {
                plot_ui.vline(VLine::new(time_playing.as_secs_f64()).width(3.0));
            }
        });
}
