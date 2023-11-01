use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn create_block(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<CreatedBlock>> {
    let mut starknet = state.api.starknet.write().await;
    starknet.create_block(None).map_err(|_| HttpApiError::CreateEmptyBlockError)?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(CreatedBlock { block_hash: block.block_hash() })),
        Err(_err) => Err(HttpApiError::CreateEmptyBlockError),
    }
}

pub(crate) async fn abort_blocks(
    Json(_data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    Err(HttpApiError::GeneralError)
}
