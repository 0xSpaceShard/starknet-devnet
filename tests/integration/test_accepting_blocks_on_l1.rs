use starknet_rs_core::types::{
    BlockId, BlockStatus, BlockTag, Felt, MaybePreConfirmedBlockWithTxHashes,
    SequencerTransactionStatus, StarknetError,
};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::errors::RpcError;

/// Returns the hash of the dummy tx
async fn send_dummy_tx(devnet: &BackgroundDevnet) -> Felt {
    devnet.mint(Felt::ONE, 1).await // dummy data
}

/// Asserts blocks and txs are accepted on L1. `block_hashes` are expected in reverse chronological
/// and `tx_hashes` in chronological order.
async fn assert_accepted_on_l1(
    devnet: &BackgroundDevnet,
    block_hashes: &[Felt],
    tx_hashes: &[Felt],
) -> Result<(), anyhow::Error> {
    for block_hash in block_hashes {
        match devnet.json_rpc_client.get_block_with_tx_hashes(BlockId::Hash(*block_hash)).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(block)) => {
                anyhow::ensure!(
                    block.status == BlockStatus::AcceptedOnL1,
                    format!(
                        "assertion `left == right` failed, left: {:?}, right: {:?}",
                        block.status,
                        BlockStatus::AcceptedOnL1
                    )
                )
            }
            other => anyhow::bail!("Unexpected block: {other:?}"),
        }
    }

    for tx_hash in tx_hashes {
        let tx_status = devnet.json_rpc_client.get_transaction_status(tx_hash).await.unwrap();
        let tx_finality_status = tx_status.finality_status();
        anyhow::ensure!(
            tx_finality_status == SequencerTransactionStatus::AcceptedOnL1,
            format!(
                "assertion `left == right` failed, left: {tx_finality_status:?}, right: {:?}",
                SequencerTransactionStatus::AcceptedOnL1
            )
        );
    }

    Ok(())
}

async fn assert_latest_accepted_on_l2(devnet: &BackgroundDevnet) -> Result<(), anyhow::Error> {
    let latest_block = devnet.get_latest_block_with_tx_hashes().await?;
    anyhow::ensure!(
        latest_block.status == BlockStatus::AcceptedOnL2,
        format!(
            "assertion `left == right` failed, left: {:?}, right: {:?}",
            latest_block.status,
            BlockStatus::AcceptedOnL2
        )
    );

    for tx_hash in latest_block.transactions {
        let tx_status = devnet.json_rpc_client.get_transaction_status(tx_hash).await?;
        let tx_finality_status = tx_status.finality_status();
        anyhow::ensure!(
            tx_finality_status == SequencerTransactionStatus::AcceptedOnL2,
            format!(
                "assertion `left == right` failed, left: {:?}, right: {:?}",
                tx_finality_status,
                SequencerTransactionStatus::AcceptedOnL2
            )
        )
    }

    Ok(())
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_id_latest() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let mut tx_hashes = vec![];
    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];
    for _ in 0..2 {
        let tx_hash = send_dummy_tx(&devnet).await;
        tx_hashes.push(tx_hash);
        let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
        block_hashes.push(block_hash);
    }

    block_hashes.reverse(); // the hashes are in reverse chronological order

    let accepted_block_hashes = devnet.accept_on_l1(&BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &tx_hashes).await.unwrap();
}

#[tokio::test]
async fn should_convert_all_txs_in_block_on_demand() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .unwrap();

    let mut tx_hashes = vec![];
    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    for _ in 0..2 {
        let tx_hash = send_dummy_tx(&devnet).await;
        tx_hashes.push(tx_hash);
    }

    let generated_block_hash = devnet.create_block().await.unwrap();
    let block_hashes = vec![generated_block_hash, origin_block_hash];

    let accepted_block_hashes = devnet.accept_on_l1(&BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &tx_hashes).await.unwrap();
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_numeric_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];

    let tx_hash = send_dummy_tx(&devnet).await;
    let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    block_hashes.push(block_hash);

    // Extra tx that won't be accepted on l1
    send_dummy_tx(&devnet).await;

    block_hashes.reverse(); // the hashes are in reverse chronological order

    let accepted_block_hashes = devnet.accept_on_l1(&BlockId::Number(1)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &[tx_hash]).await.unwrap();
    assert_latest_accepted_on_l2(&devnet).await.unwrap();
}

#[tokio::test]
async fn should_convert_accepted_on_l2_with_hash_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let origin_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    let mut block_hashes = vec![origin_block_hash];

    let tx_hash = send_dummy_tx(&devnet).await;
    let block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
    block_hashes.push(block_hash);

    // Extra tx that won't be accepted on l1
    send_dummy_tx(&devnet).await;

    block_hashes.reverse(); // the hashes are in reverse chronological order

    let accepted_block_hashes = devnet.accept_on_l1(&BlockId::Hash(block_hash)).await.unwrap();
    assert_eq!(accepted_block_hashes, block_hashes);

    assert_accepted_on_l1(&devnet, &block_hashes, &[tx_hash]).await.unwrap();
    assert_latest_accepted_on_l2(&devnet).await.unwrap();
}

#[tokio::test]
async fn block_tag_l1_accepted_should_return_last_accepted_on_l1() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let origin_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let created_block_hash = devnet.create_block().await.unwrap();

    let accepted = devnet.accept_on_l1(&BlockId::Hash(created_block_hash)).await.unwrap();
    assert_eq!(accepted, vec![created_block_hash, origin_block.block_hash]);

    let l1_accepted_block = devnet.get_l1_accepted_block_with_tx_hashes().await.unwrap();
    assert_eq!(l1_accepted_block.block_hash, created_block_hash);

    // Creating a new block should not affect the response
    let latest_block_hash = devnet.create_block().await.unwrap();
    let l1_accepted_block = devnet.get_l1_accepted_block_with_tx_hashes().await.unwrap();
    assert_eq!(l1_accepted_block.block_hash, created_block_hash);
    assert_ne!(l1_accepted_block.block_hash, latest_block_hash);
}

#[tokio::test]
async fn origin_block_should_be_acceptable_on_l1() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let mut origin_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let accepted = devnet.accept_on_l1(&BlockId::Hash(origin_block.block_hash)).await.unwrap();
    assert_eq!(accepted, vec![origin_block.block_hash]);

    let l1_accepted_block = devnet.get_l1_accepted_block_with_tx_hashes().await.unwrap();

    origin_block.status = BlockStatus::AcceptedOnL1;
    assert_eq!(origin_block, l1_accepted_block);
}

#[tokio::test]
async fn block_tag_l1_accepted_should_error_if_no_blocks_accepted_on_l1() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let err = devnet.get_l1_accepted_block_with_tx_hashes().await.unwrap_err();
    match err.downcast::<ProviderError>().unwrap() {
        ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
        other => panic!("Unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn should_fail_if_accepting_already_accepted_on_l1() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    devnet.accept_on_l1(&BlockId::Tag(BlockTag::Latest)).await.unwrap();
    let err = devnet.accept_on_l1(&BlockId::Tag(BlockTag::Latest)).await.unwrap_err();
    assert_eq!(
        err,
        RpcError { code: -1, message: "Block already accepted on L1".into(), data: None }
    );
}

#[tokio::test]
async fn should_fail_if_accepting_pre_confirmed() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .unwrap();

    let tx_hash = send_dummy_tx(&devnet).await;

    let err = devnet.accept_on_l1(&BlockId::Tag(BlockTag::PreConfirmed)).await.unwrap_err();
    assert_eq!(
        err,
        RpcError {
            code: -1,
            message: "Pre-confirmed block cannot be accepted on L1".into(),
            data: None
        }
    );

    let tx = devnet.json_rpc_client.get_transaction_status(tx_hash).await.unwrap();
    assert_eq!(tx.finality_status(), SequencerTransactionStatus::PreConfirmed);

    // Assert genesis still latest
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_eq!(latest_block.block_number, 0);

    // Assert pre_confirmed intact
    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    assert_eq!(pre_confirmed_block.transactions, vec![tx_hash]);
}

#[tokio::test]
async fn should_fail_if_accepting_rejected() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .unwrap();

    send_dummy_tx(&devnet).await;
    let aborted_blocks = devnet.abort_blocks(&BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(aborted_blocks.len(), 1);
    let aborted_block_hash = aborted_blocks[0];

    let err = devnet.accept_on_l1(&BlockId::Hash(aborted_block_hash)).await.unwrap_err();
    assert_eq!(err, RpcError { code: -1, message: "No block found".into(), data: None });
}

#[tokio::test]
async fn should_fail_if_invalid_block_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    for unacceptable_block_id in [BlockId::Hash(Felt::ONE), BlockId::Number(1)] {
        let err = devnet.accept_on_l1(&unacceptable_block_id).await.unwrap_err();
        assert_eq!(err, RpcError { code: -1, message: "No block found".into(), data: None });
    }
}
