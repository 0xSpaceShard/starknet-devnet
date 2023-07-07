use axum::Json;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::HttpApiResult;

pub(crate) async fn create_block() -> HttpApiResult<Json<CreatedBlock>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn abort_blocks(
    Json(_data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    Err(HttpApiError::GeneralError)
}
