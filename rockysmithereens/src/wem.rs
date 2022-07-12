use anyhow::Result;
use bevy::{
    asset::{AddAsset, AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    audio::{Audio, AudioOutput, Decodable},
    prelude::{App, CoreStage, IntoExclusiveSystem, Plugin},
    reflect::TypeUuid,
};
use rodio_wem::WemDecoder;

/// Bevy source for playing wem files.
#[derive(TypeUuid)]
#[uuid = "af6466c2-a9f4-11eb-bcbc-0242ac130002"]
pub struct WemSource {
    pub decoder: WemDecoder,
}

impl Decodable for WemSource {
    type Decoder = WemDecoder;
    type DecoderItem = <Self::Decoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        // TODO: remove this clone
        self.decoder.clone()
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
            let decoder = WemDecoder::new(bytes)?;

            let source = WemSource { decoder };

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
            .init_resource::<Audio<WemSource>>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                bevy::audio::play_queued_audio_system::<WemSource>.exclusive_system(),
            )
            .init_asset_loader::<WemLoader>();
    }
}
