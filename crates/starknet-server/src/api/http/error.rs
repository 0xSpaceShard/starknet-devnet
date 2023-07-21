use axum::response::IntoResponse;
use axum::Json;
use hyper::StatusCode;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpApiError {
    #[error("Path not found")]
    PathNotFound,
    #[error("General error")]
    GeneralError,
    #[error("Minting error: {msg}")]
    MintingError { msg: String },
}

impl IntoResponse for HttpApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            HttpApiError::PathNotFound => {
                (StatusCode::BAD_REQUEST, String::from("path is missing"))
            }
            HttpApiError::GeneralError => {
                (StatusCode::INTERNAL_SERVER_ERROR, String::from("general error"))
            }
            HttpApiError::MintingError { msg } => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
