use std::sync::Arc;

use bevy::{
    audio::{Audio, Decodable},
    ecs::event::EventReader,
    prelude::{AssetServer, Assets, Res, ResMut},
};
use rodio_wem::WemDecoder;

use crate::{asset::RocksmithAsset, event::LoadedEvent, State};

/// Listen for the loaded event.
pub fn loaded_listener(
    mut events: EventReader<LoadedEvent>,

    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    for _ in events.iter() {
        let music = asset_server.load("audio/windows/2147314675.wem");
        dbg!(&music);
        audio.play(music);
    }
}
