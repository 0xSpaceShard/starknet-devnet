pub mod endpoints;
pub mod error;
#[allow(unused)]
pub(crate) mod models;

use self::error::HttpApiError;

/// Helper type for the result of the http api calls and reducing typing HttpApiError
pub type HttpApiResult<T> = Result<T, HttpApiError>;
