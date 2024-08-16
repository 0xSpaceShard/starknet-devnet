use crate::api::http::error::HttpApiError;
use crate::api::http::models::{IncreaseTime, IncreaseTimeResponse, SetTime, SetTimeResponse};
use crate::api::http::HttpApiResult;
use crate::api::json_rpc::error::StrictRpcResult;
use crate::api::json_rpc::DevnetResponse;
use crate::api::Api;

pub(crate) async fn set_time_impl(api: &Api, data: SetTime) -> StrictRpcResult {
    let mut starknet = api.starknet.lock().await;
    let generate_block = data.generate_block.unwrap_or(true);

    starknet.set_time(data.time, generate_block)?;

    let block_hash = if generate_block {
        let last_block = starknet.get_latest_block()?;
        Some(last_block.block_hash())
    } else {
        None
    };

    Ok(DevnetResponse::SetTime(SetTimeResponse { block_timestamp: data.time, block_hash }).into())
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
