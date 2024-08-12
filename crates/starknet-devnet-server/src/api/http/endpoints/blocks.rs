use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AbortedBlocks, AbortingBlocks, CreatedBlock};
use crate::api::http::HttpApiResult;
use crate::api::Api;

pub(crate) async fn create_block_impl(api: &Api) -> HttpApiResult<CreatedBlock> {
    let mut starknet = api.starknet.lock().await;
    starknet
        .create_block()
        .map_err(|err| HttpApiError::CreateEmptyBlockError { msg: err.to_string() })?;

    let last_block = starknet.get_latest_block();
    match last_block {
        Ok(block) => Ok(CreatedBlock { block_hash: block.block_hash() }),
        Err(err) => Err(HttpApiError::CreateEmptyBlockError { msg: err.to_string() }),
    }
}

pub(crate) async fn abort_blocks_impl(
    api: &Api,
    data: AbortingBlocks,
) -> HttpApiResult<AbortedBlocks> {
    let mut starknet = api.starknet.lock().await;

    let aborted = starknet
        .abort_blocks(From::from(data.starting_block_id))
        .map_err(|err| HttpApiError::BlockAbortError { msg: (err.to_string()) })?;

    Ok(AbortedBlocks { aborted })
}
