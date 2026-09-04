use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("{0}: not a recognised font container")]
    UnknownFormat(PathBuf),
    #[error("font parse error: {0}")]
    Parse(String),
    #[error("WOFF decode error: {0}")]
    Woff(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

impl From<read_fonts::ReadError> for Error {
    fn from(e: read_fonts::ReadError) -> Self {
        Error::Parse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
