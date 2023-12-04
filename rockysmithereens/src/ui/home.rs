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
pub struct HomescreenGui {
    /// Window size.
    window_size: Extent2<f32>,
    /// Button reference for opening the file.
    open_file_button_node: ButtonRef,
    /// Actual Gui.
    gui: Gui,
}

impl HomescreenGui {
    /// Create the home screen.
    pub fn new(window_size: Extent2<f32>) -> Self {
        // Create a new Gui
        let mut gui = GuiBuilder::new(Style {
            // Use the amount of pixels as the calculation size
            size: Size::from_points(window_size.w, window_size.h),
            ..Default::default()
        });

        // Create a button attached to the root
        let open_file_button_node = gui
            .add_widget::<ButtonRef>(
                Button {
                    label: Some("Load .psarc file".to_string()),
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
            open_file_button_node,
        }
    }

    /// Update the Gui.
    ///
    /// Returns whether to trigger the open window.
    pub fn update(&mut self, input: &Input, mouse_pos: Option<Vec2<usize>>) -> bool {
        // Update the GUI to fill the screen
        self.gui.update_layout(Vec2::zero(), self.window_size.as_());

        // Update the button manually
        let button: &mut Button = self.gui.widget_mut(self.open_file_button_node).unwrap();

        // Handle the button press
        button.update(input, mouse_pos)
    }

    /// Render the Gui.
    pub fn render(&mut self, canvas: &mut Canvas) {
        // Reset the canvas
        canvas.fill(0xFFFFFFFF);

        // Render the button manually
        let button: &Button = self.gui.widget(self.open_file_button_node).unwrap();
        button.render(canvas);
    }
}
