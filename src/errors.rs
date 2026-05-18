use thiserror::Error;
#[derive(Error, Debug)]
pub enum FicDataError {
    #[error("Selector error: {0}")]
    SelectorError(String),
    #[error("regex error: {0}")]
    RegexError(String),
    #[error("serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("database error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("database connection error: {0}")]
    DieselConnectionError(#[from] diesel::ConnectionError),
    #[error("{0}")]
    GenericError(String),
}
