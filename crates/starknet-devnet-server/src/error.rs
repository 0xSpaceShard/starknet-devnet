#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    AxumError(#[from] axum::Error),
}

pub type ServerResult<T, E = Error> = Result<T, E>;
