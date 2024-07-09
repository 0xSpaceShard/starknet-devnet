use axum::extract::State;
use axum::Json;
use starknet_rs_core::types::BlockId;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;

pub async fn create_block(
    State(state): State<HttpApiHandler>,
) -> HttpApiResult<Json<CreatedBlock>> {
    create_block_impl(&state.api).await.map(Json::from)
}

pub(crate) async fn create_block_impl(api: &Api) -> HttpApiResult<CreatedBlock> {
    let mut starknet = api.starknet.write().await;
    starknet
        .create_block_dump_event(None)
        .map_err(|err| HttpApiError::CreateEmptyBlockError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(CreatedBlock { block_hash: block.block_hash() }),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub async fn abort_blocks(
    State(state): State<HttpApiHandler>,
    Json(data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    abort_blocks_impl(&state.api, data).await.map(Json::from)
}

pub(crate) async fn abort_blocks_impl(
    api: &Api,
    data: AbortingBlocks,
) -> HttpApiResult<AbortedBlocks> {
    let mut starknet = api.starknet.write().await;

    if data.starting_block_hash.is_some() && data.starting_block_id.is_some() {
        return Err(HttpApiError::BlockAbortError {
            msg: "Both starting_block_id and legacy starting_block_hash provided. Please provide \
                  one or the other."
                .to_string(),
        });
    }

    let block_id = match data.starting_block_id {
        Some(block_id) => From::from(block_id),
        None => match data.starting_block_hash {
            Some(block_hash) => BlockId::Hash(block_hash.into()),
            None => {
                return Err(HttpApiError::BlockAbortError {
                    msg: "Either starting_block_id or starting_block_hash must be provided"
                        .to_string(),
                });
            }
        },
    };
    let aborted = starknet
        .abort_blocks(block_id)
        .map_err(|err| HttpApiError::BlockAbortError { msg: (err.to_string()) })?;

    Ok(AbortedBlocks { aborted })
}
