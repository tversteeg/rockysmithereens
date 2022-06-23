use std::{
    fmt::{Debug, Display},
    string::FromUtf8Error,
};

use nom::{error::VerboseError, Err};
use rodio::decoder::DecoderError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WemError>;

#[derive(Debug, Error)]
pub enum WemError {
    #[error("missing data at '{0}'")]
    MissingData(String),
    #[error("input bytes are not vorbis")]
    NotVorbis,
    #[error("rodio decoder error: {0}")]
    Rodio(#[from] DecoderError),
    #[error("parsing error: {0}")]
    Nom(String),
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
