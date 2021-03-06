use std::fmt::Debug;

use lewton::{audio::AudioReadError, header::HeaderReadError};
use nom::{error::VerboseError, Err};
use rodio::decoder::DecoderError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WemError>;

#[derive(Debug, Error)]
pub enum WemError {
    #[error("{0} is corrupt")]
    Corrupt(String),
    #[error("missing data at '{0}'")]
    MissingData(String),
    #[error("missing chunk '{0}'")]
    MissingChunk(String),
    #[error("input bytes are not vorbis")]
    NotVorbis,
    #[error("rodio decoder error: {0}")]
    Rodio(#[from] DecoderError),
    #[error("lewton header reading error: {0}")]
    LewtonHeadRead(#[from] HeaderReadError),
    #[error("audio reading error: {0}")]
    AudioReadError(#[from] AudioReadError),
    #[error("parsing error: {0}")]
    Nom(String),
    #[error("writing bytes error: {0}")]
    WritingBytes(#[from] std::io::Error),
}

impl<T: Debug> From<Err<VerboseError<T>>> for WemError {
    fn from(err: Err<VerboseError<T>>) -> Self {
        match err {
            Err::Incomplete(_) => todo!(),
            Err::Error(err) => Self::Nom(format!("{:?}", err)),
            Err::Failure(_) => todo!(),
        }
    }
}
