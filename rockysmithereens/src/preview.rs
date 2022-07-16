use std::{fs::File, io::Read, path::PathBuf};

use anyhow::Result;
use bevy::prelude::{App, Commands, Component, Entity, Plugin, Query, SystemSet};
use rockysmithereens_parser::SongFile;

use crate::Phase;

/// Files from a folder that have not been loaded yet.
#[derive(Component)]
pub struct UnloadedPath(pub PathBuf);

/// A small preview of a loaded file.
#[derive(Component)]
pub struct Preview {
    pub artist: String,
    pub album: String,
    pub song: String,
    pub length: f32,
    pub path: PathBuf,
}

/// Bevy plugin for showing previews of files.
#[derive(Debug)]
pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(Phase::SongSelectionMenu).with_system(load_preview),
        );
    }
}

/// Load a single unloaded path and parse it as a preview.
fn load_preview(mut commands: Commands, query: Query<(Entity, &UnloadedPath)>) {
    if let Some((entity, UnloadedPath(path))) = query.iter().next() {
        bevy::log::debug!("Parsing {:?}", path);

        // Already remove the file so if something goes wrong it won't be tried every iteration
        commands.entity(entity).despawn();

        let _result: Result<()> = (|| {
            // Read the .psarc file
            let mut file = File::open(path)?;
            let metadata = std::fs::metadata(path)?;

            // Read the bytes
            let mut bytes = vec![0; metadata.len() as usize];
            file.read_exact(&mut bytes)?;

            let songfile = SongFile::parse(&bytes)?;

            let attributes = songfile.manifests[0].attributes();

            // Insert the preview
            commands.spawn().insert(Preview {
                artist: attributes.artist().to_string(),
                album: attributes.album().to_string(),
                song: attributes.name().to_string(),
                length: attributes.song_length,
                path: path.clone(),
            });

            Ok(())
        })();

        // TODO: do something with the result
    }
}
