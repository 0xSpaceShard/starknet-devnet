#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    HyperError(#[from] hyper::Error),
}

pub type ServerResult<T, E = Error> = Result<T, E>;
