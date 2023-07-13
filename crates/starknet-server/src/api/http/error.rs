use axum::response::IntoResponse;
use axum::Json;
use hyper::StatusCode;
use serde_json::json;

#[derive(Debug)]
pub enum HttpApiError {
    PathNotFound,
    GeneralError,
}

impl IntoResponse for HttpApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            HttpApiError::PathNotFound => (StatusCode::BAD_REQUEST, "path is missing"),
            HttpApiError::GeneralError => (StatusCode::INTERNAL_SERVER_ERROR, "general error"),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
