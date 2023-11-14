use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{IncreaseTimeResponse, SetTimeResponse, Time};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn set_time(
    Json(data): Json<Time>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<SetTimeResponse>> {
    let mut starknet = state.api.starknet.write().await;
    starknet
        .set_time(data.time)
        .map_err(|err| HttpApiError::BlockSetTimeError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(SetTimeResponse {
            block_timestamp: block.timestamp().0,
            block_hash: block.block_hash(),
        })),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub(crate) async fn increase_time(
    Json(data): Json<Time>,
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
