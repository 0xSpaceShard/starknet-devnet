use serde_json::json;
use starknet_rs_core::types::{
    BlockId, BlockStatus, BlockTag, Felt, MaybePreConfirmedBlockWithTxHashes,
    SequencerTransactionStatus,
};
use starknet_rs_providers::Provider;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::errors::RpcError;

async fn accept_on_l1(
    devnet: &BackgroundDevnet,
    starting_block_id: &BlockId,
) -> Result<Vec<Felt>, RpcError> {
    let accepted_block_hashes_raw = devnet
        .send_custom_rpc("devnet_acceptOnL1", json!({ "starting_block_id" : starting_block_id }))
        .await?;

    let accepted_block_hashes =
        serde_json::from_value(accepted_block_hashes_raw["accepted"].clone()).unwrap();
    Ok(accepted_block_hashes)
}

/// Asserts blocks and txs are accepted on L1. `block_hashes` are expected in reverse chronological
/// and `tx_hashes` in chronological order.
async fn assert_accepted_on_l1(
    devnet: &BackgroundDevnet,
    block_hashes: &[Felt],
    tx_hashes: &[Felt],
) {
    for block_hash in block_hashes {
        match devnet.json_rpc_client.get_block_with_tx_hashes(BlockId::Hash(*block_hash)).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(block)) => {
                assert_eq!(block.status, BlockStatus::AcceptedOnL1)
            }
            other => panic!("Unexpected block: {other:?}"),
        }
    }

    for tx_hash in tx_hashes {
        let tx_status = devnet.json_rpc_client.get_transaction_status(tx_hash).await.unwrap();
        assert_eq!(tx_status.finality_status(), SequencerTransactionStatus::AcceptedOnL1);
    }
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_id_latest() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let mut tx_hashes = vec![];
    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];
    for _ in 0..2 {
        let tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data
        tx_hashes.push(tx_hash);
        let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
        block_hashes.push(block_hash);
    }

    block_hashes.reverse(); // the hashes are in reverse chronological order TODO?

    let accepted_block_hashes =
        accept_on_l1(&devnet, &BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &tx_hashes).await;
}

#[tokio::test]
async fn should_convert_all_txs_in_block_on_demand() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .unwrap();

    let mut tx_hashes = vec![];
    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    for _ in 0..2 {
        let tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data
        tx_hashes.push(tx_hash);
    }

    let generated_block_hash = devnet.create_block().await.unwrap();
    let block_hashes = vec![generated_block_hash, origin_block_hash];

    let accepted_block_hashes =
        accept_on_l1(&devnet, &BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &tx_hashes).await;
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_numeric_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];

    let tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data
    let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    block_hashes.push(block_hash);

    // Extra tx that won't be accepted on l1
    let extra_tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data

    block_hashes.reverse(); // the hashes are in reverse chronological order TODO?

    let accepted_block_hashes = accept_on_l1(&devnet, &BlockId::Number(1)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &[tx_hash]).await;

    // Assert latest block and tx untouched
    assert_eq!(
        devnet.get_latest_block_with_tx_hashes().await.unwrap().status,
        BlockStatus::AcceptedOnL2,
    );
    assert_eq!(
        devnet
            .json_rpc_client
            .get_transaction_status(extra_tx_hash)
            .await
            .unwrap()
            .finality_status(),
        SequencerTransactionStatus::AcceptedOnL2
    )
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_hash_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];

    let tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data
    let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    block_hashes.push(block_hash);

    // Extra tx that won't be accepted on l1
    let extra_tx_hash = devnet.mint(Felt::ONE, 1).await; // dummy data

    block_hashes.reverse(); // the hashes are in reverse chronological order TODO?

    let accepted_block_hashes = accept_on_l1(&devnet, &BlockId::Hash(block_hash)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &[tx_hash]).await;

    // Assert latest block and tx untouched
    assert_eq!(
        devnet.get_latest_block_with_tx_hashes().await.unwrap().status,
        BlockStatus::AcceptedOnL2,
    );
    assert_eq!(
        devnet
            .json_rpc_client
            .get_transaction_status(extra_tx_hash)
            .await
            .unwrap()
            .finality_status(),
        SequencerTransactionStatus::AcceptedOnL2
    )
}

#[tokio::test]
async fn should_fail_if_accepting_already_accepted_on_l1() {
    todo!()
}

#[tokio::test]
async fn should_fail_if_accepting_pre_confirmed() {
    todo!()
}

#[tokio::test]
async fn should_fail_if_accepting_rejected() {
    todo!()
}

#[tokio::test]
async fn should_fail_if_invalid_block_id() {
    todo!("block hash, block number")
}
