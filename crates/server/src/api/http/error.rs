use axum::response::IntoResponse;
use axum::Json;
use hyper::StatusCode;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpApiError {
    #[error("General error")]
    GeneralError,
    #[error("Minting error: {msg}")]
    MintingError { msg: String },
    #[error("The file does not exist")]
    FileNotFound,
    #[error("The dump operation failed: {msg}")]
    DumpError { msg: String },
    #[error("The load operation failed")]
    LoadError,
    #[error("The re-execution operation failed")]
    ReExecutionError,
    #[error("The creation of empty block failed: {msg}")]
    CreateEmptyBlockError { msg: String },
    #[error("The set time operation failed: {msg}")]
    BlockSetTimeError { msg: String },
    #[error("The increase time operation failed: {msg}")]
    BlockIncreaseTimeError { msg: String },
    #[error("Could not restart: {msg}")]
    RestartError { msg: String },
    #[error("Messaging error: {msg}")]
    MessagingError { msg: String },
    #[error("Invalid value: {msg}")]
    InvalidValueError { msg: String },
}

impl IntoResponse for HttpApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            err @ HttpApiError::GeneralError => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::FileNotFound => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::DumpError { msg: _ } => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::LoadError => (StatusCode::BAD_REQUEST, err.to_string()),
            err @ HttpApiError::ReExecutionError => (StatusCode::BAD_REQUEST, err.to_string()),
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
