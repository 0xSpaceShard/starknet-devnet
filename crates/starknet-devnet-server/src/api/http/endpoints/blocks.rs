use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub async fn create_block(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<CreatedBlock>> {
    let mut starknet = state.api.starknet.write().await;
    starknet
        .create_block_dump_event(None)
        .map_err(|err| HttpApiError::CreateEmptyBlockError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(CreatedBlock { block_hash: block.block_hash() })),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub async fn abort_blocks(
    Json(data): Json<AbortingBlocks>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    let mut starknet = state.api.starknet.write().await;
    starknet.abort_blocks(data.starting_block_hash);

    let aborted = vec![data.starting_block_hash];
    Ok(Json(AbortedBlocks { aborted }))
}
