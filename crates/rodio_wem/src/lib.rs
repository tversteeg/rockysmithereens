mod error;

use std::{
    io::{Read, Seek},
    time::Duration,
};

use rodio::{Decoder, Source};

use crate::error::{Result, WemError};

/// Extract a Wwise .wem file payload into a rodio [`VorbisDecoder`].
pub fn vorbis_from_wem<R>(data: R) -> Result<Decoder<R>>
where
    R: Read + Seek + Send + Sync + 'static,
{
    let decoder = Decoder::new_vorbis(data)?;

    Ok(decoder)
}
