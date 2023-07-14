pub(crate) mod endpoints;
pub(crate) mod error;
#[allow(unused)]
mod models;

use self::error::HttpApiError;
use super::Api;

/// Helper type for the result of the http api calls and reducing typing HttpApiError
type HttpApiResult<T> = Result<T, HttpApiError>;

/// This object will be used as a shared state between HTTP calls.
#[derive(Clone)]
pub struct HttpApiHandler {
    pub api: Api,
}
