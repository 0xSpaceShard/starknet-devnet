use serde_json::json;
use starknet_rs_core::types::{BlockId, BlockTag, Felt, StarknetError};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::errors::RpcError;
use crate::common::utils::{FeeUnit, assert_contains, to_hex_felt};

static DUMMY_ADDRESS: u128 = 1;
static DUMMY_AMOUNT: u128 = 1;

async fn abort_blocks_error(
    devnet: &BackgroundDevnet,
    starting_block_id: &BlockId,
    expected_message_substring: &str,
) {
    let aborted_blocks_error = devnet
        .send_custom_rpc("devnet_abortBlocks", json!({ "starting_block_id" : starting_block_id }))
        .await
        .unwrap_err();
    assert_contains(&aborted_blocks_error.message, expected_message_substring);
}

async fn assert_block_aborted(devnet: &BackgroundDevnet, block_hash: &Felt) {
    let err = devnet
        .send_custom_rpc(
            "starknet_getBlockWithTxHashes",
            json!({ "block_id": {"block_hash": to_hex_felt(block_hash)} }),
        )
        .await
        .unwrap_err();

    assert_eq!(err, RpcError { code: 24, message: "Block not found".into(), data: None })
}

async fn assert_txs_aborted(devnet: &BackgroundDevnet, tx_hashes: &[Felt]) {
    for tx_hash in tx_hashes {
        match devnet.json_rpc_client.get_transaction_by_hash(tx_hash).await {
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => (),
            other => panic!("Unexpected tx response: {other:?}"),
        }
    }
}

#[tokio::test]
async fn abort_latest_block_with_hash() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let genesis_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;

    let new_block_hash = devnet.create_block().await.unwrap();
    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(new_block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![new_block_hash]);

    // Check if the genesis block is still ACCEPTED_ON_L2
    let genesis_block_after_abort = &devnet
        .send_custom_rpc(
            "starknet_getBlockWithTxHashes",
            json!({ "block_id": {"block_hash": to_hex_felt(&genesis_block_hash)} }),
        )
        .await
        .unwrap();
    assert_eq!(genesis_block_after_abort["status"], "ACCEPTED_ON_L2".to_string());

    assert_block_aborted(&devnet, &new_block_hash).await;

    abort_blocks_error(
        &devnet,
        &BlockId::Hash(genesis_block_hash),
        "Genesis block can't be aborted",
    )
    .await;
}

#[tokio::test]
async fn abort_two_blocks() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let first_block_hash = devnet.create_block().await.unwrap();
    let second_block_hash = devnet.create_block().await.unwrap();

    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(first_block_hash)).await.unwrap();
    assert_eq!(json!(aborted_blocks), json!([second_block_hash, first_block_hash]));

    assert_block_aborted(&devnet, &first_block_hash).await;
    assert_block_aborted(&devnet, &second_block_hash).await;
}

#[tokio::test]
async fn abort_block_with_transaction() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let mint_hash = devnet.mint(Felt::ONE, 100).await;

    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let aborted_blocks =
        devnet.abort_blocks(&BlockId::Hash(latest_block.block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![latest_block.block_hash]);

    assert_block_aborted(&devnet, &latest_block.block_hash).await;
    assert_txs_aborted(&devnet, &[mint_hash]).await;
}

#[tokio::test]
async fn query_aborted_block_by_number_should_fail() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let new_block_hash = devnet.create_block().await.unwrap();
    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(new_block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![new_block_hash]);
    assert_block_aborted(&devnet, &new_block_hash).await;

    let rpc_error = devnet
        .send_custom_rpc(
            "starknet_getBlockWithTxHashes",
            json!({ "block_id": {"block_number": 1} }),
        )
        .await
        .unwrap_err();
    assert_eq!(rpc_error.message, "Block not found")
}

#[tokio::test]
async fn block_abortion_should_affect_state() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    // State setup
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let first_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let second_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let balance_before_abortion =
        devnet.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri).await.unwrap();
    assert_eq!(balance_before_abortion.to_string(), (2 * DUMMY_AMOUNT).to_string());

    // Block abortion - latest block only
    let aborted_blocks =
        devnet.abort_blocks(&BlockId::Hash(second_block.block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![second_block.block_hash]);

    // Aborting the second mint should halve the balance
    let balance =
        devnet.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri).await.unwrap();
    assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(first_block.block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![first_block.block_hash]);

    // Aborting the first mint should make the balance 0
    let balance_after_all_aborted =
        devnet.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri).await.unwrap();
    assert_eq!(balance_after_all_aborted.to_string(), "0");

    // Re-gain balance
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let balance =
        devnet.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri).await.unwrap();
    assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());
}

#[tokio::test]
async fn block_abortion_should_affect_block_number() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    /// Assert `devnet` currently has `latest` and `pre_confirmed` as its block numbers.
    async fn assert_block_number(devnet: &BackgroundDevnet, latest: u64, pre_confirmed: u64) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, latest);
        let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
        assert_eq!(pre_confirmed_block.block_number, pre_confirmed);
    }

    // State setup
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let first_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let second_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    assert_block_number(&devnet, 2, 3).await;

    // Block abortion - latest block only
    let aborted_blocks =
        devnet.abort_blocks(&BlockId::Hash(second_block.block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![second_block.block_hash]);
    assert_block_number(&devnet, 1, 2).await;

    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(first_block.block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![first_block.block_hash]);
    assert_block_number(&devnet, 0, 1).await;

    // Re-gain balance
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    assert_block_number(&devnet, 1, 2).await;
}

#[tokio::test]
async fn abort_blocks_without_state_archive_capacity() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let new_block_hash = devnet.create_block().await.unwrap();
    abort_blocks_error(
        &devnet,
        &BlockId::Hash(new_block_hash),
        "The abort blocks feature requires state-archive-capacity set to full",
    )
    .await;
}

#[tokio::test]
async fn abort_same_block_twice() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let first_block_hash = devnet.create_block().await.unwrap();
    let second_block_hash = devnet.create_block().await.unwrap();

    let aborted_blocks = devnet.abort_blocks(&BlockId::Hash(first_block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![second_block_hash, first_block_hash]);

    abort_blocks_error(&devnet, &BlockId::Hash(first_block_hash), "No block found").await;
    abort_blocks_error(&devnet, &BlockId::Hash(second_block_hash), "No block found").await;
}

#[tokio::test]
/// The purpose of this test to prevent a bug which overwrote the list of aborted blocks with newly
/// aborted blocks, thus forgetting old abortions.
async fn abort_same_block_twice_if_blocks_aborted_on_two_occasions() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let first_block_hash = devnet.create_block().await.unwrap();
    let second_block_hash = devnet.create_block().await.unwrap();

    let first_aborted_blocks =
        devnet.abort_blocks(&BlockId::Hash(second_block_hash)).await.unwrap();
    assert_eq!(first_aborted_blocks, vec![second_block_hash]);

    let second_aborted_blocks =
        devnet.abort_blocks(&BlockId::Hash(first_block_hash)).await.unwrap();
    assert_eq!(second_aborted_blocks, vec![first_block_hash]);

    abort_blocks_error(&devnet, &BlockId::Hash(first_block_hash), "No block found").await;
    abort_blocks_error(&devnet, &BlockId::Hash(second_block_hash), "No block found").await;
}

#[tokio::test]
async fn abort_block_after_fork() {
    let origin_devnet: BackgroundDevnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let fork_devnet = origin_devnet.fork_with_full_state_archive().await.unwrap();

    let fork_block_hash = fork_devnet.create_block().await.unwrap();

    let aborted_blocks = fork_devnet.abort_blocks(&BlockId::Hash(fork_block_hash)).await.unwrap();
    assert_eq!(aborted_blocks, vec![fork_block_hash]);

    abort_blocks_error(&fork_devnet, &BlockId::Hash(fork_block_hash), "No block found").await;
}

#[tokio::test]
async fn abort_latest_blocks() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    for _ in 0..3 {
        devnet.create_block().await.unwrap();
    }
    for _ in 0..3 {
        devnet.abort_blocks(&BlockId::Tag(BlockTag::Latest)).await.unwrap();
    }
    abort_blocks_error(&devnet, &BlockId::Tag(BlockTag::Latest), "Genesis block can't be aborted")
        .await;
}

#[tokio::test]
async fn abort_pending_block() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--state-archive-capacity",
        "full",
        "--block-generation-on",
        "demand",
    ])
    .await
    .expect("Could not start Devnet");

    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    devnet.create_block().await.unwrap();
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let pending_balance = devnet
        .get_balance_by_tag(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri, BlockTag::PreConfirmed)
        .await
        .unwrap();
    assert_eq!(pending_balance, (2 * DUMMY_AMOUNT).into());

    devnet.abort_blocks(&BlockId::Tag(BlockTag::PreConfirmed)).await.unwrap();
    let latest_balance =
        devnet.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Fri).await.unwrap();
    assert_eq!(latest_balance, DUMMY_AMOUNT.into());
}
