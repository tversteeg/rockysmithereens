use std::time::Duration;

use crate::{Phase, State, LOADED_SONG};
use bevy::{
    pbr::{PbrBundle, StandardMaterial},
    prelude::{
        shape::Cube, App, Assets, Color, Commands, Component, Mesh, Plugin, Res, ResMut, SystemSet,
        Transform,
    },
};

/// How high each note will get.
pub const Y_NOTE_SCALE: f32 = 1.2;
/// The z distance for each second for a note, determines the speed.
pub const Z_NOTE_SCALE: f32 = 20.0;

/// All strings.
pub const STRINGS: [StringNumber; 6] = [
    StringNumber::String1,
    StringNumber::String2,
    StringNumber::String3,
    StringNumber::String4,
    StringNumber::String5,
    StringNumber::String6,
];

/// Marker for a regular note.
#[derive(Debug, Component)]
pub struct Note;

/// When the note must be triggered.
///
/// Float represents the amount of seconds.
#[derive(Debug, Component)]
pub struct TriggerTime(f32);

impl TriggerTime {
    /// Calculate the time relative to the current time.
    ///
    /// Result is in seconds.
    pub fn relative_time(&self, time_playing: Duration) -> f32 {
        self.0 - time_playing.as_secs_f32()
    }
}

/// How long the note must be played.
#[derive(Debug, Component)]
pub struct Sustain(f32);

/// To which fret the note belongs.
///
/// `None` means it's an open string.
#[derive(Debug, Component, Clone, Copy)]
pub enum Fret {
    Open,
    Fret(u8),
}

impl Fret {
    /// Get the X position for this fret.
    pub fn x(self) -> Option<f32> {
        match self {
            Fret::Open => None,
            Fret::Fret(fret) => Some(fret as f32 * 2.0),
        }
    }
}

impl From<u8> for Fret {
    fn from(fret: u8) -> Self {
        match fret {
            0 => Self::Open,
            fret => Self::Fret(fret),
        }
    }
}

/// On which string the note must be played.
#[derive(Debug, Component, Clone, Copy)]
#[repr(u8)]
pub enum StringNumber {
    String1 = 0,
    String2 = 1,
    String3 = 2,
    String4 = 3,
    String5 = 4,
    String6 = 5,
}

impl StringNumber {
    /// Get the vertical position in the 3D world for this string.
    pub fn y(self) -> f32 {
        (Self::String6 as u8 as f32 * Y_NOTE_SCALE - self as u8 as f32) * Y_NOTE_SCALE
    }
}

impl From<u8> for StringNumber {
    fn from(string: u8) -> Self {
        match string {
            0 => Self::String1,
            1 => Self::String2,
            2 => Self::String3,
            3 => Self::String4,
            4 => Self::String5,
            5 => Self::String6,
            _ => panic!("Unrecognized string"),
        }
    }
}

impl From<StringNumber> for Color {
    fn from(string: StringNumber) -> Self {
        match string {
            StringNumber::String1 => Color::RED,
            StringNumber::String2 => Color::YELLOW,
            StringNumber::String3 => Color::ALICE_BLUE,
            StringNumber::String4 => Color::ORANGE,
            StringNumber::String5 => Color::GREEN,
            StringNumber::String6 => Color::CYAN,
        }
    }
}

/// Bevy plugin for the notes.
#[derive(Debug)]
pub struct NotePlugin;

impl Plugin for NotePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(Phase::Loading).with_system(inject_notes));
    }
}

/// Convert the loaded notes to bevy entities.
#[profiling::function]
fn inject_notes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<State>,
    mut phase: ResMut<bevy::prelude::State<Phase>>,
) {
    if let Some(song) = &*LOADED_SONG.lock().unwrap() {
        // TODO: handle errors
        let parsed_song = song.parse_song_info(state.current_song.unwrap()).unwrap();

        for note in parsed_song.notes_iter() {
            // Spawn the notes
            let mut entity = commands.spawn();
            entity.insert(Note);

            entity.insert(TriggerTime(note.time));

            let string = StringNumber::from(note.string);
            entity.insert(string);

            // The fret
            let fret = Fret::from(note.fret);
            entity.insert(fret);

            if let Some(x) = fret.x() {
                // The mesh
                entity.insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(Cube { size: 1.0 })),
                    // Color of the mesh is based on the string
                    material: materials.add(Color::from(string).into()),
                    transform: Transform::from_xyz(x, string.y(), note.time * Z_NOTE_SCALE),
                    ..Default::default()
                });
            }
        }

        // Add it as a resource
        commands.insert_resource(parsed_song);

        // We are ready to play
        phase.overwrite_set(Phase::Playing).unwrap();
    }
}
