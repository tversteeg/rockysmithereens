use std::fmt::{Debug, Display};

use nom::error::VerboseError;
use nom::Err;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ArchiveReadError>;

#[derive(Debug, Error)]
pub enum ArchiveReadError {
    #[error("unrecognized file")]
    UnrecognizedFile,
    #[error("unsupported version")]
    UnsupportedVersion,
    #[error("corrupt file")]
    Corrupt,
    #[error("file does not exist")]
    FileDoesNotExist,
    #[error("parsing error")]
    Nom(String),
}

impl<T: Debug> From<Err<VerboseError<T>>> for ArchiveReadError {
    fn from(err: Err<VerboseError<T>>) -> Self {
        match err {
            Err::Incomplete(_) => todo!(),
            Err::Error(err) => ArchiveReadError::Nom(format!("{:?}", err)),
            Err::Failure(_) => todo!(),
        }
    }
}
