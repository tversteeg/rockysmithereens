use std::fmt::{Debug, Display};

use rodio::decoder::DecoderError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WemError>;

#[derive(Debug, Error)]
pub enum WemError {
    #[error("missing data at '{0}'")]
    MissingData(String),
    #[error("rodio decoder error: {0}")]
    Rodio(#[from] DecoderError),
}
