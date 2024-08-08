#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    AxumError(#[from] axum::Error),
    #[error("Failed conversion: {0}")]
    ConversionError(String),
}

pub type ServerResult<T, E = Error> = Result<T, E>;
