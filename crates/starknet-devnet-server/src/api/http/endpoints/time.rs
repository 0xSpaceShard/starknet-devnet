use axum::extract::State;
use axum::Json;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{IncreaseTime, IncreaseTimeResponse, SetTime, SetTimeResponse};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;

pub async fn set_time(
    State(state): State<HttpApiHandler>,
    Json(data): Json<SetTime>,
) -> HttpApiResult<Json<SetTimeResponse>> {
    set_time_impl(&state.api, data).await.map(Json::from)
}

pub async fn increase_time(
    State(state): State<HttpApiHandler>,
    Json(data): Json<IncreaseTime>,
) -> HttpApiResult<Json<IncreaseTimeResponse>> {
    increase_time_impl(&state.api, data).await.map(Json::from)
}

pub(crate) async fn set_time_impl(api: &Api, data: SetTime) -> HttpApiResult<SetTimeResponse> {
    let mut starknet = api.starknet.lock().await;
    let generate_block = data.generate_block.unwrap_or(true);

    starknet
        .set_time(data.time, generate_block)
        .map_err(|err| HttpApiError::BlockSetTimeError { msg: err.to_string() })?;

    if !generate_block {
        return Ok(SetTimeResponse { block_timestamp: data.time, block_hash: None });
    }

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(SetTimeResponse {
            block_timestamp: block.timestamp().0,
            block_hash: Some(block.block_hash()),
        }),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub(crate) async fn increase_time_impl(
    api: &Api,
    data: IncreaseTime,
) -> HttpApiResult<IncreaseTimeResponse> {
    let mut starknet = api.starknet.lock().await;
    starknet
        .increase_time(data.time)
        .map_err(|err| HttpApiError::BlockIncreaseTimeError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(IncreaseTimeResponse {
            timestamp_increased_by: data.time,
            block_hash: block.block_hash(),
        }),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}
