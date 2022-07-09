use std::{ops::Deref, time::Duration};

use bevy::{
    audio::{Audio, AudioSink},
    core::Time,
    input::Input,
    prelude::{
        App, AssetServer, Assets, Commands, Handle, KeyCode, Plugin, Res, ResMut, SystemSet,
    },
};
use bevy_egui::{
    egui::{
        plot::{Plot, Points, Text, Value, Values},
        CentralPanel, Color32, TopBottomPanel, Vec2,
    },
    EguiContext,
};

use crate::{wem::WemSource, Phase, State, LOADED_SONG};

/// Time between this and the current time before a note is spawned.
const NOTE_SPAWN_TIME: f32 = 20.0;

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
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(show_notes))
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

/// Show the notes.
pub fn show_notes(
    mut context: ResMut<EguiContext>,
    music_controller: Res<MusicController>,
    level: Res<Level>,
) {
    let time_playing_secs = music_controller.time_playing.as_secs_f32();

    // Get the notes that will be played soon
    let notes =
        level.notes_between_time_iter(time_playing_secs, time_playing_secs + NOTE_SPAWN_TIME);

    TopBottomPanel::bottom("notes").show(context.ctx_mut(), |ui| {
        ui.label("Notes");

        Plot::new("notes_plot")
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .include_x(0.0)
            .include_x(NOTE_SPAWN_TIME)
            .include_y(-1.0)
            .include_y(7.0)
            .show_x(false)
            .show_y(false)
            .height(300.0)
            .show(ui, |plot_ui| {
                notes.for_each(|note| {
                    plot_ui.text(
                        Text::new(
                            Value::new(note.time - time_playing_secs, note.string),
                            format!("{}", note.fret),
                        )
                        .color(match note.string {
                            // E
                            0 => Color32::from_rgb(0xFF, 0x00, 0x00),
                            // A
                            1 => Color32::from_rgb(0xFF, 0xFF, 0x00),
                            // D
                            2 => Color32::from_rgb(0x00, 0x00, 0xFF),
                            // G
                            3 => Color32::from_rgb(0xFF, 0x99, 0x00),
                            // B
                            4 => Color32::from_rgb(0x00, 0xFF, 0x00),
                            // e
                            5 => Color32::from_rgb(0xFF, 0x00, 0xFF),
                            _ => Color32::from_rgb(0xFF, 0xFF, 0xFF),
                        }),
                    )
                });
            });
    });
}

/// Load the song.
pub fn load_song(
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

/// Load the song XML.
pub fn load_song_xml(mut commands: Commands, state: Res<State>) {
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        // TODO: handle errors
        let xml = song.parse_song_info(state.current_song.unwrap()).unwrap();

        // Find the highest difficulty of the song
        let highest_difficulty = xml.highest_difficulty().unwrap();

        // Get the level for the specified difficulty
        let level = xml
            .into_level_with_difficulty(highest_difficulty / 2)
            .unwrap();

        // Register the level
        commands.insert_resource(Level(level));
    }
}
