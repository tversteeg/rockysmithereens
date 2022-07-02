use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use bevy::{
    asset::{AssetIo, AssetIoError, BoxedFuture},
    prelude::{App, AssetServer, Assets, EventReader, EventWriter, Plugin, Res, ResMut},
    tasks::{IoTaskPool, TaskPool},
};
use rockysmithereens_parser::SongFile;

use crate::{
    asset::RocksmithAsset,
    event::{LoadedEvent, StartEvent},
    State,
};

// TODO: add this to the filesystem struct, needs an bevy update where the current asset loader can
// be accessed in a mutable way
lazy_static::lazy_static! {
/// The song state.
    pub static ref LOADED_SONG: Mutex<Option<SongFile>> = Mutex::new(None);
}

/// Rocksmith archive representing a bevy virtual file system.
pub struct Filesystem {
    /// The regular bevy filesystem IO is used.
    file: Box<dyn AssetIo>,
}

impl AssetIo for Filesystem {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        if let Some(song) = &*LOADED_SONG.lock().unwrap() {
            // Load a file from the archive
            let bytes = song
                .read_file(path.to_str().expect("could not convert path to psarc path"))
                // TODO: use proper errors
                .expect("could not read path in psarc file");

            Box::pin(async move { Ok(bytes) })
        } else {
            self.file.load_path(path)
        }
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        self.file.read_directory(path)
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.file.is_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        self.file.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        self.file.watch_for_changes()
    }
}

/// The plugin for loading the files from the archive.
#[derive(Debug)]
pub struct FilesystemPlugin;

impl Plugin for FilesystemPlugin {
    fn build(&self, app: &mut App) {
        // Setup the default asset io for file loading
        let asset_io = Filesystem {
            file: bevy::asset::create_platform_default_asset_io(app),
        };

        // Get the default task pool
        let task_pool = app
            .world
            .get_resource::<IoTaskPool>()
            .expect("`IoTaskPool` resource not found")
            .0
            .clone();

        app.insert_resource(AssetServer::new(asset_io, task_pool))
            .add_system(song_loaded_listener);
    }
}

/// Event listener for switching to the song virtual filesystem.
pub fn song_loaded_listener(
    mut start_events: EventReader<StartEvent>,
    mut loaded_events: EventWriter<LoadedEvent>,
    state: ResMut<State>,
    mut rocksmith_assets: ResMut<Assets<RocksmithAsset>>,
) {
    for _ in start_events.iter() {
        // Move the asset to this filesystem
        let asset = rocksmith_assets.remove(&state.handle);
        if let Some(file) = asset {
            *LOADED_SONG.lock().unwrap() = Some(file.0);

            loaded_events.send_default();
        }
    }
}
