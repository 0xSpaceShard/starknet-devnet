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
    #[error("The dump operation failed: {msg}")]
    DumpError { msg: String },
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
            HttpApiError::GeneralError(err) => (StatusCode::BAD_REQUEST, err),
            err @ HttpApiError::DumpError { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::MessagingError { .. } => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::InvalidValueError { .. } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
