use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{IncreaseTime, SetTime, Time};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn set_time(
    Json(data): Json<Time>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<SetTime>> {
    let mut starknet = state.api.starknet.write().await;
    starknet.set_time(data.time);
    starknet.create_empy_block().map_err(|_| HttpApiError::GeneralError); // TODO: change to something else

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(SetTime {
            block_timestamp: block.timestamp().0,
            block_hash: block.block_hash(),
        })),
        Err(err) => {
            Err(HttpApiError::GeneralError) // TODO: change error to something else
        }
    }
}

pub(crate) async fn increase_time(
    Json(data): Json<Time>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<IncreaseTime>> {
    let mut starknet = state.api.starknet.write().await;
    starknet.increase_time(data.time);
    starknet.create_empy_block().map_err(|_| HttpApiError::GeneralError); // TODO: change to something else

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(Json(IncreaseTime {
            timestamp_increased_by: block.timestamp().0,
            block_hash: block.block_hash(),
        })),
        Err(err) => {
            Err(HttpApiError::GeneralError) // TODO: change to something else
        }
    }
}
