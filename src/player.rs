use bevy::{
    ecs::event::EventReader,
    prelude::{Assets, ResMut},
};

use crate::{asset::RocksmithAsset, event::StartEvent, State};

/// Listen for the start event.
pub fn start_listener(
    mut events: EventReader<StartEvent>,
    state: ResMut<State>,
    rocksmith_assets: ResMut<Assets<RocksmithAsset>>,
) {
    for _ in events.iter() {
        let asset = rocksmith_assets.get(&state.handle);
        if let Some(file) = asset {
            if let Some(current_song) = state.current_song {
                // Load the bytes of the ogg song
                let bytes = file.0.ogg(current_song).expect("loading song file");
            }
        }
    }
}
