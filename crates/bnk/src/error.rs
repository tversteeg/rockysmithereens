use std::fmt::Debug;

use nom::{error::VerboseError, Err};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, BnkError>;

#[derive(Debug, Error)]
pub enum BnkError {
    #[error("{0} is corrupt")]
    Corrupt(String),
    #[error("section '{0}' is missing")]
    MissingSection(String),
    #[error("parsing error: {0}")]
    Nom(String),
}

impl<T: Debug> From<Err<VerboseError<T>>> for BnkError {
    fn from(err: Err<VerboseError<T>>) -> Self {
        match err {
            Err::Incomplete(_) => todo!(),
            Err::Error(err) => Self::Nom(format!("{:?}", err)),
            Err::Failure(_) => todo!(),
        }
    }
}
