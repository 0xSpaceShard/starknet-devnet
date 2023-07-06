use axum::Json;

use crate::api::http::{
    error::HttpApiError,
    models::{AbortedBlocks, AbortingBlocks, CreatedBlock},
    HttpApiResult,
};

pub(crate) async fn create_block() -> HttpApiResult<Json<CreatedBlock>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn abort_blocks(
    Json(_data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    Err(HttpApiError::GeneralError)
}
