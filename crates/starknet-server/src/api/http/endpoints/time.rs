use axum::Json;

use crate::api::http::{error::HttpApiError, models::Time, HttpApiResult};

pub(crate) async fn set_time(Json(_data): Json<Time>) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn increase_time(Json(_data): Json<Time>) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}
