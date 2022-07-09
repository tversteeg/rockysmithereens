use std::fmt::Debug;

use bnk::BnkError;
use psarc::ArchiveReadError;
use quick_xml::de::DeError;
use rodio_wem::WemError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, RocksmithArchiveError>;

#[derive(Debug, Error)]
pub enum RocksmithArchiveError {
    #[error("parsing playstation archive")]
    Archive(#[from] ArchiveReadError),
    #[error("xml read error")]
    Xml(#[from] DeError),
    #[error("json read error")]
    Json(#[from] serde_json::Error),
    #[error("decoding wem file: {0}")]
    Wem(#[from] WemError),
    #[error("decoding bnk file: {0}")]
    Bnk(#[from] BnkError),
    #[error("playstation archive is not rocksmith specific")]
    NotARocksmitheFile,
    #[error("missing data at '{0}'")]
    MissingData(String),
    #[error("urn path '{0}' is invalid")]
    InvalidUrnPath(String),
    #[error("level with difficulty {0} not found")]
    NoLevelWithDifficulty(u8),
}
