use std::{ops::Deref, time::Duration};

use bevy::{
    audio::{Audio, AudioSink},
    core::Time,
    input::Input,
    prelude::{
        App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, ResMut, SystemSet,
    },
};

use crate::{wem::WemSource, Phase, State, LOADED_SONG};

/// Time between this and the current time before a note is spawned.
pub const NOTE_SPAWN_TIME: f32 = 20.0;

/// Music player event handler.
#[derive(Debug, Default)]
pub struct MusicController {
    // Handle to the audio sink to pause the music.
    sink: Handle<AudioSink>,
    // Handle to the music source.
    source: Handle<WemSource>,
    // How far we are along with the song.
    pub time_playing: Duration,
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

/// Level resource.
#[derive(Debug, Default)]
pub struct Level(rockysmithereens_parser::song_xml::Level);

impl Deref for Level {
    type Target = rockysmithereens_parser::song_xml::Level;

    fn deref(&self) -> &Self::Target {
        &self.0
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
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(load_song_xml))
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(pause))
            .add_system_set(
                SystemSet::on_update(Phase::Playing).with_system(update_playing_duration),
            );
    }
}

/// Pause the music.
#[profiling::function]
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
#[profiling::function]
pub fn update_playing_duration(
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

/// Load the song.
#[profiling::function]
pub fn load_song(
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

/// Load the song XML.
#[profiling::function]
pub fn load_song_xml(mut commands: Commands, state: Res<State>) {
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        // TODO: handle errors
        let xml = song.parse_song_info(state.current_song.unwrap()).unwrap();

        /*
        // TODO: update based on selected difficulty
        // Find the highest difficulty of the song
        let highest_difficulty = xml.highest_difficulty().unwrap();

        // Get the level for the specified difficulty
        let level = xml.into_level_with_difficulty(highest_difficulty).unwrap();

        // Register the level
        commands.insert_resource(Level(level));
        */
        commands.insert_resource(xml);
    }
}
