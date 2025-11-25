use starknet_rs_core::types::{BlockId, BlockTag, StarknetError};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;

#[tokio::test]
async fn test_invalid_block() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let res = devnet
        .json_rpc_client
        .get_block_transaction_count(BlockId::Number(9000000000))
        .await
        .unwrap_err();
    match res {
        ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
        _ => panic!("Invalid error: {res:?}"),
    }
}

#[tokio::test]
async fn test_valid_block() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    devnet
        .json_rpc_client
        .get_block_transaction_count(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();
}
