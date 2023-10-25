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
    #[error("The dump operation failed")]
    DumpError { msg: String },
    #[error("The load operation failed")]
    LoadError,
    #[error("The re-execution operation failed")]
    ReExecutionError,
    #[error("The creation of empty block failed")]
    CreateEmptyBlockError,
    #[error("The set time operation failed")]
    SetTimeError,
    #[error("The increase time operation failed")]
    IncreaseTimeError,
    #[error("The get of latest block failed")]
    GetLatestBlockError,
}

impl IntoResponse for HttpApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            HttpApiError::GeneralError => {
                (StatusCode::INTERNAL_SERVER_ERROR, String::from("general error"))
            }
            HttpApiError::FileNotFound => {
                (StatusCode::BAD_REQUEST, String::from("file does not exist"))
            }
            err @ HttpApiError::DumpError { msg: _ } => (StatusCode::BAD_REQUEST, err.to_string()),
            HttpApiError::LoadError => {
                (StatusCode::BAD_REQUEST, String::from("load operation failed"))
            }
            HttpApiError::ReExecutionError => {
                (StatusCode::BAD_REQUEST, String::from("re-execution operation failed"))
            }
            HttpApiError::CreateEmptyBlockError => {
                (StatusCode::BAD_REQUEST, String::from("creation of empty block failed"))
            }
            HttpApiError::SetTimeError => {
                (StatusCode::BAD_REQUEST, String::from("set time operation failed"))
            }
            HttpApiError::IncreaseTimeError => {
                (StatusCode::BAD_REQUEST, String::from("increase time operation failed"))
            }
            HttpApiError::GetLatestBlockError => {
                (StatusCode::BAD_REQUEST, String::from("get of latest block failed"))
            }
            err @ HttpApiError::MintingError { msg: _ } => {
                (StatusCode::BAD_REQUEST, err.to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
