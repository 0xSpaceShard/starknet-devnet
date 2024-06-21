pub mod endpoints;
pub mod error;
#[allow(unused)]
pub(crate) mod models;

use self::error::HttpApiError;
use super::Api;
use crate::ServerConfig;

/// Helper type for the result of the http api calls and reducing typing HttpApiError
type HttpApiResult<T> = Result<T, HttpApiError>;

/// This object will be used as a shared state between HTTP calls.
#[derive(Clone)]
pub struct HttpApiHandler {
    pub api: Api,
    pub server_config: ServerConfig,
}
