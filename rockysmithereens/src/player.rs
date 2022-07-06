use std::time::Duration;

use bevy::{
    audio::{Audio, AudioSink},
    core::Time,
    input::Input,
    prelude::{
        App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, ResMut, SystemSet,
    },
};

use crate::{wem::WemSource, Phase, LOADED_SONG};

/// Music player event handler.
#[derive(Debug, Default)]
pub struct MusicController {
    // Handle to the audio sink to pause the music.
    sink: Handle<AudioSink>,
    // How far we are along with the song.
    time_playing: Duration,
}

impl MusicController {
    /// Start a new controller with the time set to zero.
    pub fn new(sink: Handle<AudioSink>) -> Self {
        Self {
            sink,
            time_playing: Duration::ZERO,
        }
    }

    /// How far we are along with the song.
    pub fn time_playing(&self) -> Duration {
        self.time_playing
    }
}

/// Bevy plugin for the audio player.
#[derive(Debug)]
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicController>()
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(loaded_listener))
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(pause))
            .add_system_set(
                SystemSet::on_update(Phase::Playing).with_system(update_playing_duration),
            );
    }
}

/// Pause the music.
pub fn pause(
    keyboard_input: Res<Input<KeyCode>>,
    audio_sinks: Res<Assets<AudioSink>>,
    music_controller: Res<MusicController>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Some(sink) = audio_sinks.get(&music_controller.sink) {
            if sink.is_paused() {
                sink.play()
            } else {
                sink.pause()
            }
        }
    }
}

/// Update the duration based on if we are playing.
pub fn update_playing_duration(
    audio_sinks: Res<Assets<AudioSink>>,
    mut music_controller: ResMut<MusicController>,
    time: Res<Time>,
) {
    if let Some(sink) = audio_sinks.get(&music_controller.sink) {
        if !sink.is_paused() {
            music_controller.time_playing += time.delta();
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
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        let music = asset_server.load(song.song_path());
        let handle = sinks.get_handle(audio.play(music));
        commands.insert_resource(MusicController::new(handle));
    }
}
