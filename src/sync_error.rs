use std::convert::Infallible;

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error(transparent)]
    ExecError(#[from] exec_rs::ExecError),
    #[error("split error")]
    SplitError,
    #[error("path deletion error ({0})")]
    PathDeletionError(String),
    #[error("error converting path to string ({0})")]
    PathConversionError(String),
    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),
    #[error("duration conversion error")]
    DurationConversionError,
    #[error(transparent)]
    Infallible(#[from] Infallible),
}
