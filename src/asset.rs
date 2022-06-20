use anyhow::Result;
use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    reflect::TypeUuid,
};
use rockysmithereens_parser::SongFile;

/// Loaded Rocksmith asset file.
#[derive(Debug, TypeUuid)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct RocksmithAsset(pub SongFile);

/// Custom asset loader to automatically load the .psarc Rocksmith files.
#[derive(Debug, Default)]
pub struct RocksmithAssetLoader;

impl AssetLoader for RocksmithAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let song = SongFile::parse(bytes)?;

            let asset = RocksmithAsset(song);

            load_context.set_default_asset(LoadedAsset::new(asset));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["psarc"]
    }
}
