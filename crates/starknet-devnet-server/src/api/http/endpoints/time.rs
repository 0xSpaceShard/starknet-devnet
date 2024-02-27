use axum::{Extension, Json};
use starknet_core::error::Error;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{IncreaseTime, IncreaseTimeResponse, SetTime, SetTimeResponse};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub async fn set_time(
    Json(data): Json<SetTime>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<SetTimeResponse>> {
    let mut starknet = state.api.starknet.write().await;
    let generate_block = data.generate_block.unwrap_or(true);

    starknet
        .set_time(data.time, generate_block)
        .map_err(|err| HttpApiError::BlockSetTimeError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(SetTimeResponse {
            block_timestamp: block.timestamp().0,
            block_hash: Some(block.block_hash()),
        })),
        Err(Error::NoBlock) => {
            // Handle case when generate_block is false and there is no latest block
            if !generate_block {
                return Ok(Json(SetTimeResponse { block_timestamp: data.time, block_hash: None }));
            }

            Err(HttpApiError::CreateEmptyBlockError { msg: Error::NoBlock.to_string() })
        }
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub async fn increase_time(
    Json(data): Json<IncreaseTime>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<IncreaseTimeResponse>> {
    let mut starknet = state.api.starknet.write().await;
    starknet
        .increase_time(data.time)
        .map_err(|err| HttpApiError::BlockIncreaseTimeError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(IncreaseTimeResponse {
            timestamp_increased_by: data.time,
            block_hash: block.block_hash(),
        })),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}
