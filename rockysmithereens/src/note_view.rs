use bevy::{
    math::Vec3,
    pbr::{PointLight, PointLightBundle},
    prelude::{
        shape::Box, AlphaMode, AmbientLight, App, Assets, BuildChildren, Camera, Camera3dBundle,
        Color, Commands, Component, DirectionalLightBundle, Mesh, ParamSet, PbrBundle, Plugin,
        Query, Res, ResMut, StandardMaterial, SystemSet, Transform, With, Without,
    },
};

use crate::{
    note::{Fret, Note, StringNumber, TriggerTime, STRINGS, Z_NOTE_SCALE},
    player::MusicController,
    Phase,
};

/// How high the camera is above the note.
pub const CAMERA_Y_OFFSET: f32 = 10.0;
/// How far away from the note the camera is.
pub const CAMERA_Z_OFFSET: f32 = 20.0;

/// Tilt for the camera.
pub const CAMERA_ANGLE: f32 = 10.0;

/// Component should transform with the camera position.
#[derive(Debug, Component, Default)]
pub struct FollowCamera(Vec3);

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
    mut camera: Query<&mut Transform, (With<Camera>, Without<Note>, Without<FollowCamera>)>,
    mut follows_camera: Query<(&mut Transform, &FollowCamera), (Without<Note>, Without<Camera>)>,
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

    let camera_zero = Vec3::new(
        closest.x,
        closest.y,
        music_controller.time_playing().as_secs_f32() * Z_NOTE_SCALE,
    );

    // Point the camera to it
    let mut transform = camera.single_mut();
    *transform = transform.with_translation(Vec3::new(
        camera_zero.x,
        camera_zero.y + CAMERA_Y_OFFSET,
        camera_zero.z - CAMERA_Z_OFFSET,
    ));

    // Move the items following the camera to it
    follows_camera
        .iter_mut()
        .for_each(|(mut transform, offset)| {
            *transform = transform.with_translation(camera_zero + offset.0)
        });
}

/// Spawn the camera entity and point it to the proper place.
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The camera
    let mut transform = Transform::identity();
    transform.rotate_y(180f32.to_radians());
    transform.rotate_x(CAMERA_ANGLE.to_radians());

    commands.spawn_bundle(Camera3dBundle {
        transform,
        ..Default::default()
    });

    // Background light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });

    // The neck meshes
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(Box::new(1000.0, 0.1, 0.1))),
            // Color of the mesh is based on the string
            material: materials.add(Color::GRAY.into()),
            ..Default::default()
        })
        .insert(FollowCamera::default())
        .with_children(|parent| {
            // The light
            parent.spawn_bundle(DirectionalLightBundle {
                transform: Transform::from_xyz(0.0, 20.0, 0.0),
                ..Default::default()
            });

            // The semi-transparent background
            parent.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(Box::new(1000.0, 6.0, 0.1))),
                // Color of the mesh is based on the string
                material: materials.add(StandardMaterial {
                    base_color: *Color::ALICE_BLUE.as_rgba().set_a(0.5),
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                }),
                transform: Transform::from_xyz(0.0, 3.0, 0.0),
                ..Default::default()
            });

            // The string lines
            STRINGS.iter().for_each(|string| {
                parent.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(Box::new(1000.0, 0.1, 0.1))),
                    // Color of the mesh is based on the string
                    material: materials.add(Color::GRAY.into()),
                    transform: Transform::from_xyz(0.0, string.y(), 0.0),
                    ..Default::default()
                });
            });
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
