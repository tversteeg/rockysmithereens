use bevy::{
    audio::{Audio, AudioSink},
    input::Input,
    prelude::{App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, SystemSet},
};

use crate::{wem::WemSource, Phase};

/// Music player event handler.
#[derive(Debug, Default)]
pub struct MusicController(Handle<AudioSink>);

/// Bevy plugin for the audio player.
#[derive(Debug)]
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicController>()
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(loaded_listener))
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(pause));
    }
}

/// Pause the music.
pub fn pause(
    keyboard_input: Res<Input<KeyCode>>,
    audio_sinks: Res<Assets<AudioSink>>,
    music_controller: Res<MusicController>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Some(sink) = audio_sinks.get(&music_controller.0) {
            if sink.is_paused() {
                sink.play()
            } else {
                sink.pause()
            }
        }
    }
}

/// Listen for the loaded event.
pub fn loaded_listener(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<Audio<WemSource>>,
    sinks: Res<Assets<AudioSink>>,
) {
    let music = asset_server.load("audio/mac/1867869353.wem");
    let handle = sinks.get_handle(audio.play(music));
    commands.insert_resource(MusicController(handle));
}
