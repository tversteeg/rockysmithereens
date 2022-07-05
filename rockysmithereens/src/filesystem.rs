use std::{
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetIo, AssetIoError, BoxedFuture},
    prelude::{App, AssetServer, Plugin},
    tasks::IoTaskPool,
};


use crate::{LOADED_SONG};

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

        app.insert_resource(AssetServer::new(asset_io, task_pool));
    }
}
