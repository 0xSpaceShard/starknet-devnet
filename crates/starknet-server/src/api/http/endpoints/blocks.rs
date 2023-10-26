use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn create_block(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<CreatedBlock>> {
    let mut starknet = state.api.starknet.write().await;
    starknet.update_pending_block_timestamp();
    starknet.create_block().map_err(|_| HttpApiError::CreateEmptyBlockError)?;
    let last_block = starknet.get_latest_block();

    match last_block {
        Ok(block) => Ok(Json(CreatedBlock {
            block_hash: block.block_hash(),
            block_timestamp: block.timestamp().0,
        })),
        Err(_err) => Err(HttpApiError::SetTimeError),
    }
}

pub(crate) async fn abort_blocks(
    Json(_data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    Err(HttpApiError::GeneralError)
}
