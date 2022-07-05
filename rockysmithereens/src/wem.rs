

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
#[uuid = "af6466c2-a9f4-11eb-bcbc-0242ac130002"]
pub struct WemSource {
    pub bytes: Vec<u8>,
}

impl Decodable for WemSource {
    type Decoder = WemDecoder;
    type DecoderItem = <Self::Decoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
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
        let source = WemSource {
            bytes: bytes.to_vec(),
        };

        load_context.set_default_asset(LoadedAsset::new(source));

        Box::pin(async move { Ok(()) })
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
            .init_resource::<Audio<WemSource>>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                bevy::audio::play_queued_audio_system::<WemSource>.exclusive_system(),
            )
            .init_asset_loader::<WemLoader>();
    }
}
