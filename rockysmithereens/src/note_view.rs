use bevy::{
    math::Vec3,
    pbr::{PointLight, PointLightBundle},
    prelude::{
        App, Camera, Commands, ParamSet, PerspectiveCameraBundle, Plugin, Query, Res, SystemSet,
        Transform, UiCameraBundle, With, Without,
    },
    render::camera::Camera3d,
};

use crate::{
    note::{Fret, Note, StringNumber, TriggerTime},
    player::MusicController,
    Phase,
};

/// Bevy plugin for the notes.
#[derive(Debug)]
pub struct NoteViewPlugin;

impl Plugin for NoteViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(Phase::Playing).with_system(update_position))
            .add_system_set(SystemSet::on_update(Phase::Playing).with_system(update_camera))
            .add_system_set(SystemSet::on_enter(Phase::Playing).with_system(setup_scene));
    }
}

/// Import the z position based on the time.
pub fn update_position(
    mut transforms: Query<(&mut Transform, &TriggerTime, &Fret, &StringNumber)>,
    music_controller: Res<MusicController>,
) {
    transforms
        .iter_mut()
        .for_each(|(mut transform, time, fret, string)| {
            if let Some(x) = fret.x() {
                *transform = Transform::from_xyz(
                    x,
                    string.y(),
                    time.relative_time(music_controller.time_playing()) * 10.0,
                )
            }
        });
}

/// Update the camera position to automatically follow the notes.
pub fn update_camera(
    notes: Query<&Transform, With<Note>>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<Note>)>,
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
    *transform = transform.looking_at(closest, Vec3::Y);
}

/// Spawn the camera entity and point it to the proper place.
pub fn setup_scene(mut commands: Commands) {
    // The camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 5.0, -10.0)
            .looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::Y),
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
}
