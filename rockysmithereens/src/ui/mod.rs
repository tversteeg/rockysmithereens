mod arrangement_select;
mod in_game;
mod notes_plot;
mod phrases_plot;
#[cfg(feature = "profile")]
mod profiling;
mod song_select;

use bevy::prelude::{App, Plugin, SystemSet};

use crate::Phase;

/// Bevy plugin for the UI.
#[derive(Debug)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(Phase::SongSelectionMenu).with_system(song_select::ui),
        )
        .add_system_set(
            SystemSet::on_update(Phase::ArrangementSelectionMenu)
                .with_system(arrangement_select::ui),
        )
        .add_system_set(SystemSet::on_update(Phase::Playing).with_system(in_game::ui))
        .add_system_set(SystemSet::on_update(Phase::Playing).with_system(notes_plot::ui));
        #[cfg(feature = "profile")]
        app.add_system(profiling::ui);
    }
}
