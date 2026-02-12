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
    #[error("{0}")]
    GenericError(String),
}
