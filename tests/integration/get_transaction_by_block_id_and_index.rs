use starknet_rs_core::types::{BlockId, BlockTag, Felt, StarknetError};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;

#[tokio::test]
async fn get_transaction_by_block_id_and_index_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let tx_hash_value = devnet.mint(Felt::ONE, 1).await;

    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 0)
        .await
        .unwrap();

    if let starknet_rs_core::types::Transaction::Invoke(
        starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
    ) = result
    {
        assert_eq!(invoke_v1.transaction_hash, tx_hash_value);
    } else {
        panic!("Could not unpack the transaction from {result:?}");
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_wrong_index() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    devnet.mint(Felt::ONE, 1).await;

    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 1)
        .await
        .unwrap_err();

    match result {
        ProviderError::StarknetError(StarknetError::InvalidTransactionIndex) => (),
        _ => panic!("Invalid error: {result:?}"),
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_wrong_block() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Number(1), 1)
        .await
        .unwrap_err();

    match result {
        ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
        _ => panic!("Invalid error: {result:?}"),
    }
}
