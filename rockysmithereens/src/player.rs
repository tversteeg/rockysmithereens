use std::time::Duration;

use bevy::{
    audio::{Audio, AudioSink},
    input::Input,
    prelude::{
        App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, ResMut, SystemSet,
    },
    time::Time,
};
use rockysmithereens_parser::level::Level;

use crate::{wem::WemSource, Phase, State, LOADED_SONG};

/// Time between this and the current time before a note is spawned.
pub const NOTE_SPAWN_TIME: f32 = 5.0;

/// Music player event handler.
#[derive(Debug, Default)]
pub struct MusicController {
    // Handle to the audio sink to pause the music.
    sink: Handle<AudioSink>,
    // Handle to the music source.
    source: Handle<WemSource>,
    // How far we are along with the song.
    time_playing: Duration,
}

impl MusicController {
    /// Start a new controller with the time set to zero.
    pub fn new(sink: Handle<AudioSink>, source: Handle<WemSource>) -> Self {
        Self {
            sink,
            source,
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
            .init_resource::<Level>()
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(load_song))
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(pause))
            .add_system_set(
                SystemSet::on_update(Phase::Playing).with_system(update_playing_duration),
            )
            .add_system_set(SystemSet::on_exit(Phase::Playing).with_system(exit));
    }
}

/// Pause the music.
#[profiling::function]
fn pause(
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
#[profiling::function]
fn update_playing_duration(
    audio_sinks: Res<Assets<AudioSink>>,
    mut music_controller: ResMut<MusicController>,
    time: Res<Time>,
    sources: Res<Assets<WemSource>>,
) {
    // Only update if we are not loading the asset
    if sources.get(&music_controller.source).is_none() {
        return;
    }

    // Update current time
    if let Some(sink) = audio_sinks.get(&music_controller.sink) {
        if !sink.is_paused() {
            music_controller.time_playing += time.delta();
        }
    }
}

/// Stop playing the song.
#[profiling::function]
fn exit(audio_sinks: Res<Assets<AudioSink>>, music_controller: Res<MusicController>) {
    // Unload the audio
    if let Some(sink) = audio_sinks.get(&music_controller.sink) {
        sink.stop()
    }

    // Unload the loaded song file
    *LOADED_SONG.lock().unwrap() = None;
}

/// Load the song.
#[profiling::function]
fn load_song(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<Audio<WemSource>>,
    sinks: Res<Assets<AudioSink>>,
) {
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        let music = asset_server.load(song.song_path());
        let handle = sinks.get_handle(audio.play(music.clone_weak()));
        commands.insert_resource(MusicController::new(handle, music));
    }
}
