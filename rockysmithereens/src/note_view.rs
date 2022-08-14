use bevy::{
    math::Vec3,
    pbr::{PointLight, PointLightBundle},
    prelude::{
        shape::Box, App, Assets, Camera, Camera3dBundle, Color, Commands, Mesh, ParamSet,
        PbrBundle, Plugin, Query, Res, ResMut, StandardMaterial, SystemSet, Transform, With,
        Without,
    },
};

use crate::{
    note::{Fret, Note, StringNumber, TriggerTime},
    player::MusicController,
    Phase,
};

/// How high the camera is above the note.
pub const CAMERA_Y_OFFSET: f32 = 2.0;
/// How far away from the note the camera is.
pub const CAMERA_Z_OFFSET: f32 = -5.0;

/// Bevy plugin for the notes.
#[derive(Debug)]
pub struct NoteViewPlugin;

impl Plugin for NoteViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(Phase::Playing).with_system(update_camera))
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(setup_scene));
    }
}

/// Update the camera position to automatically follow the notes.
pub fn update_camera(
    notes: Query<&Transform, With<Note>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<Note>)>,
    music_controller: Res<MusicController>,
) {
    // Get the closest note
    let closest = notes
        .iter()
        .min_by_key(|note| {
            let z = note.translation.z;
            if z >= 0.0 {
                z as u32
            } else {
                // Ignore notes that have already been played
                u32::MAX
            }
        })
        .map(|closest| closest.translation)
        .unwrap_or(Vec3::ZERO);

    // Point the camera to it
    let mut transform = camera.single_mut();
    *transform = transform.with_translation(Vec3::new(
        closest.x,
        closest.y + CAMERA_Y_OFFSET,
        music_controller.time_playing().as_secs_f32() - CAMERA_Z_OFFSET,
    ));
}

/// Spawn the camera entity and point it to the proper place.
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, CAMERA_Y_OFFSET, CAMERA_Z_OFFSET)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    // The light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    // The fret meshes
    for fret in 0..24 {
        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(Box::new(0.1, 0.1, 10000.0))),
            // Color of the mesh is based on the string
            material: materials.add(Color::GRAY.into()),
            transform: Transform::from_xyz(Fret::from(fret).x().unwrap_or(0.0), 0.0, 0.0),
            ..Default::default()
        });
    }
}
