use crate::api::Api;
use crate::api::http::models::{IncreaseTime, IncreaseTimeResponse, SetTime, SetTimeResponse};
use crate::api::json_rpc::DevnetResponse;
use crate::api::json_rpc::error::StrictRpcResult;

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

pub(crate) async fn increase_time_impl(api: &Api, data: IncreaseTime) -> StrictRpcResult {
    let mut starknet = api.starknet.lock().await;
    starknet.increase_time(data.time)?;

    let last_block = starknet.get_latest_block()?;

    Ok(DevnetResponse::IncreaseTime(IncreaseTimeResponse {
        timestamp_increased_by: data.time,
        block_hash: last_block.block_hash(),
    })
    .into())
}
