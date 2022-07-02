use anyhow::Result;
use bevy::{
    asset::{AddAsset, AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    audio::{Audio, AudioOutput, Decodable},
    prelude::{App, CoreStage, IntoExclusiveSystem, Plugin},
    reflect::TypeUuid,
};
use rodio_wem::WemDecoder;

/// Bevy source for playing wem files.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "40cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct WemSource {
    pub bytes: Vec<u8>,
}

impl AsRef<[u8]> for WemSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Decodable for WemSource {
    type Decoder = WemDecoder;
    type DecoderItem = i16;

    fn decoder(&self) -> Self::Decoder {
        dbg!("bla");
        // TODO: handle errors
        WemDecoder::new(&self.bytes).unwrap()
    }
}

/// Bevy loader for loading and playing wem files.
#[derive(Debug, Default)]
pub struct WemLoader;

impl AssetLoader for WemLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            dbg!("bla");
            let source = WemSource {
                bytes: bytes.to_vec(),
            };

            load_context.set_default_asset(LoadedAsset::new(source));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wem"]
    }
}

/// Bevy plugin for playing wem files.
#[derive(Debug)]
pub struct WemPlugin;

impl Plugin for WemPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AudioOutput<WemSource>>()
            .add_asset::<WemSource>()
            .init_asset_loader::<WemLoader>()
            .init_resource::<Audio<WemSource>>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                bevy::audio::play_queued_audio_system::<WemSource>.exclusive_system(),
            );
    }
}
