use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use thiserror::Error;

use crate::api::json_rpc::WILDCARD_RPC_ERROR_CODE;
use crate::rpc_core::error::RpcError;

#[derive(Error, Debug)]
pub enum HttpApiError {
    #[error("{0}")]
    GeneralError(String),
    #[error("Minting error: {msg}")]
    MintingError { msg: String },
    #[error("The file does not exist")]
    FileNotFound,
    #[error("The dump operation failed: {msg}")]
    DumpError { msg: String },
    #[error("The load operation failed: {0}")]
    LoadError(String),
    #[error("The re-execution operation failed: {0}")]
    ReExecutionError(String),
    #[error("The creation of empty block failed: {msg}")]
    CreateEmptyBlockError { msg: String },
    #[error("The set time operation failed: {msg}")]
    BlockSetTimeError { msg: String },
    #[error("The increase time operation failed: {msg}")]
    BlockIncreaseTimeError { msg: String },
    #[error("Block abortion failed: {msg}")]
    BlockAbortError { msg: String },
    #[error("Could not restart: {msg}")]
    RestartError { msg: String },
    #[error("Messaging error: {msg}")]
    MessagingError { msg: String },
    #[error("Invalid value: {msg}")]
    InvalidValueError { msg: String },
}

impl HttpApiError {
    pub fn http_api_error_to_rpc_error(&self) -> RpcError {
        let error_message = self.to_string();
        let error_rpc_code =
            crate::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE);
        RpcError { code: error_rpc_code, message: error_message.into(), data: None }
    }
}

impl IntoResponse for HttpApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            HttpApiError::GeneralError(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::FileNotFound => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::DumpError { msg: _ } => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::LoadError(_) => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::BlockAbortError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            err @ HttpApiError::ReExecutionError(_) => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::CreateEmptyBlockError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            err @ HttpApiError::BlockSetTimeError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            err @ HttpApiError::BlockIncreaseTimeError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            err @ HttpApiError::MintingError { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::RestartError { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            err @ HttpApiError::MessagingError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            err @ HttpApiError::InvalidValueError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
