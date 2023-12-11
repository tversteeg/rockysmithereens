use std::time::Duration;

use pixel_game_lib::{
    canvas::Canvas,
    gui::{
        button::{Button, ButtonRef},
        label::{Label, LabelRef},
        Gui, GuiBuilder, Widget,
    },
    vek::{Extent2, Vec2},
    window::Input,
};
use taffy::{prelude::Size, style::Style};

/// Gui state for the homescreen.
pub struct PlayingGui {
    /// Window size.
    window_size: Extent2<f32>,
    /// Label reference for time of the song.
    playing_label_node: LabelRef,
    /// Actual Gui.
    gui: Gui,
}

impl PlayingGui {
    /// Create the home screen.
    pub fn new(window_size: Extent2<f32>) -> Self {
        // Create a new Gui
        let mut gui = GuiBuilder::new(Style {
            // Use the amount of pixels as the calculation size
            size: Size::from_points(window_size.w, window_size.h),
            ..Default::default()
        });

        // Create a button attached to the root
        let playing_label_node = gui
            .add_widget::<LabelRef>(
                Label {
                    label: "00:00".to_string(),
                    ..Default::default()
                },
                Style {
                    size: Size::from_points(300.0, 20.0),
                    ..Default::default()
                },
                gui.root(),
            )
            .unwrap();

        let gui = gui.build();

        Self {
            gui,
            window_size,
            playing_label_node,
        }
    }

    /// Update the Gui.
    ///
    /// Returns whether to trigger the open window.
    pub fn update(
        &mut self,
        time_elapsed: Duration,
        total_duration: Duration,
        input: &Input,
        mouse_pos: Option<Vec2<usize>>,
    ) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        // Update the GUI to fill the screen
        self.gui.update_layout(Vec2::zero(), self.window_size.as_());

        // Update the label manually
        let label: &mut Label = self.gui.widget_mut(self.playing_label_node).unwrap();
        label.label = format!(
            "{:02}:{:02}/{:02}:{:02}",
            time_elapsed.as_secs() / 60,
            time_elapsed.as_secs() % 60,
            total_duration.as_secs() / 60,
            total_duration.as_secs() % 60
        );
    }

    /// Render the Gui.
    pub fn render(&mut self, canvas: &mut Canvas) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        // Render the button manually
        let label: &mut Label = self.gui.widget_mut(self.playing_label_node).unwrap();
        label.render(canvas);
    }
}
