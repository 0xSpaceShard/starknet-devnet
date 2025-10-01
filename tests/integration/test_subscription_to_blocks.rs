use std::collections::HashMap;
use std::time::Duration;

use serde_json::json;
use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_rs_providers::Provider;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{
    SubscriptionId, assert_no_notifications, receive_notification, subscribe_new_heads, unsubscribe,
};

async fn receive_block(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    subscription_id: SubscriptionId,
) -> Result<serde_json::Value, anyhow::Error> {
    receive_notification(ws, "starknet_subscriptionNewHeads", subscription_id).await
}

#[tokio::test]
async fn subscribe_to_new_block_heads_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();

    // test with multiple blocks created, number 0 was origin, so we start at 1
    for block_i in 1..=2 {
        let created_block_hash = devnet.create_block().await.unwrap();

        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
        assert_eq!(notification_block["block_number"], json!(block_i));
    }
}

#[tokio::test]
async fn should_not_receive_block_notification_if_not_subscribed() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn multiple_block_subscribers_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let n_subscribers = 5;

    let mut subscribers = HashMap::new();
    for _ in 0..n_subscribers {
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();
        subscribers.insert(subscription_id, ws);
    }

    assert_eq!(subscribers.len(), n_subscribers); // assert all IDs are different

    let created_block_hash = devnet.create_block().await.unwrap();

    for (subscription_id, mut ws) in subscribers {
        let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
        assert_eq!(notification_block["block_number"], json!(1));
    }
}

#[tokio::test]
async fn subscription_to_an_old_block_by_number_should_notify_of_all_blocks_up_to_latest() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let n_blocks = 5;
    for _ in 0..n_blocks {
        devnet.create_block().await.unwrap();
    }

    // request notifications for all blocks starting with genesis
    let subscription_id =
        subscribe_new_heads(&mut ws, json!({ "block_id": BlockId::Number(0) })).await.unwrap();

    for block_i in 0..=n_blocks {
        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(block_i));
    }

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn subscription_to_an_old_block_by_hash_should_notify_of_all_blocks_up_to_latest() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let genesis_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let n_blocks = 5;
    for _ in 0..n_blocks {
        devnet.create_block().await.unwrap();
    }

    // request notifications for all blocks starting with genesis
    let subscription_id =
        subscribe_new_heads(&mut ws, json!({ "block_id": BlockId::Hash(genesis_block.block_hash)}))
            .await
            .unwrap();

    let starting_block = 0;
    for block_i in starting_block..=n_blocks {
        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(block_i));
    }

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn assert_latest_block_is_default() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws_latest, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut ws_default, _) = connect_async(devnet.ws_url()).await.unwrap();

    // create two subscriptions: one to latest, one without block (thus defaulting)
    let subscription_id_latest =
        subscribe_new_heads(&mut ws_latest, json!({ "block_id": "latest" })).await.unwrap();

    let subscription_id_default = subscribe_new_heads(&mut ws_default, json!({})).await.unwrap();

    assert_ne!(subscription_id_latest, subscription_id_default);

    devnet.create_block().await.unwrap();

    let notification_block_latest =
        receive_block(&mut ws_latest, subscription_id_latest).await.unwrap();
    assert_no_notifications(&mut ws_latest).await.unwrap();

    let notification_block_default =
        receive_block(&mut ws_default, subscription_id_default).await.unwrap();
    assert_no_notifications(&mut ws_default).await.unwrap();

    assert_eq!(notification_block_latest, notification_block_default);
}

#[tokio::test]
async fn test_multiple_subscribers_one_unsubscribes() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let n_subscribers = 3;

    let mut subscribers = HashMap::new();
    for _ in 0..n_subscribers {
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();
        subscribers.insert(subscription_id, ws);
    }

    assert_eq!(subscribers.len(), n_subscribers); // assert all IDs are different

    // randomly choose one subscriber for unsubscription
    let unsubscriber_id = subscribers.keys().next().cloned().expect("Should have at least one");

    // unsubscribe
    let mut unsubscriber_ws = subscribers.remove(&unsubscriber_id).unwrap();
    let unsubscription_resp = unsubscribe(&mut unsubscriber_ws, unsubscriber_id).await.unwrap();
    assert_eq!(unsubscription_resp, json!({ "jsonrpc": "2.0", "id": 0, "result": true }));

    // create block and assert only subscribers are notified
    let created_block_hash = devnet.create_block().await.unwrap();

    for (subscription_id, mut ws) in subscribers {
        let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
    }

    assert_no_notifications(&mut unsubscriber_ws).await.unwrap();
}

#[tokio::test]
async fn test_unsubscribing_invalid_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let unsubscription_resp = unsubscribe(&mut ws, "123".to_string()).await.unwrap();

    assert_eq!(
        unsubscription_resp,
        json!({
            "jsonrpc": "2.0",
            "id": 0,
            "error": {
                "code": 66,
                "message": "Invalid subscription id",
            }
        })
    );
}

#[tokio::test]
async fn read_only_methods_do_not_generate_notifications() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    subscribe_new_heads(&mut ws, json!({})).await.unwrap();

    devnet
        .json_rpc_client
        .get_class_hash_at(BlockId::Tag(BlockTag::Latest), ETH_ERC20_CONTRACT_ADDRESS)
        .await
        .unwrap();

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn test_notifications_in_block_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();

    let dummy_address = 0x1;
    devnet.mint(dummy_address, 1).await;

    assert_no_notifications(&mut ws).await.unwrap();

    let created_block_hash = devnet.create_block().await.unwrap();

    let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
    assert_eq!(notification_block["block_hash"], json!(created_block_hash));
    assert_eq!(notification_block["block_number"], json!(1));
}

#[tokio::test]
async fn test_notifications_on_periodic_block_generation() {
    let interval = 3;
    let devnet_args = ["--block-generation-on", &interval.to_string()];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();

    // this assertion is skipped due to CI instability
    // assert_no_notifications(&mut ws).await;

    // should be enough time for Devnet to mine a single new block
    tokio::time::sleep(Duration::from_secs(interval + 1)).await;

    let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
    assert_eq!(notification_block["block_number"], json!(1));

    // this assertion is skipped due to CI instability
    // assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_fork_subscription_to_blocks_starting_from_origin() {
    // Setup original devnet with some blocks
    let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

    // Create some blocks in origin
    let n_origin_blocks = 3;
    let mut origin_block_hashes = Vec::new();
    for _ in 0..n_origin_blocks {
        origin_block_hashes.push(origin_devnet.create_block().await.unwrap());
    }

    // Fork the devnet
    let fork_devnet = origin_devnet.fork().await.unwrap();

    // Create some blocks in the forked devnet before subscribing
    let n_fork_blocks_before_subscription = 2;
    let mut fork_block_hashes_before_subscription = Vec::new();
    for _ in 0..n_fork_blocks_before_subscription {
        let block_hash = fork_devnet.create_block().await.unwrap();
        fork_block_hashes_before_subscription.push(block_hash);
    }

    // Subscribe to blocks starting from genesis (block 0)
    let (mut ws, _) = connect_async(fork_devnet.ws_url()).await.unwrap();
    let subscription_id =
        subscribe_new_heads(&mut ws, json!({ "block_id": BlockId::Number(0) })).await.unwrap();

    // We should receive notifications for all blocks from origin (0 to n_origin_blocks)
    for block_i in 0..=n_origin_blocks {
        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(block_i));

        // Check if the block hash matches for non-genesis blocks
        if block_i > 0 {
            let expected_hash = origin_block_hashes[block_i - 1];
            assert_eq!(notification_block["block_hash"], json!(expected_hash));
        }
    }

    // Devnet creates an empty block after forking, so we receive one more block notification
    let empty_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
    let empty_block_number = n_origin_blocks + 1;
    assert_eq!(empty_block["block_number"], json!(empty_block_number));

    // We should also receive notifications for blocks created in the forked devnet before
    // subscription
    for (i, expected_hash) in fork_block_hashes_before_subscription.iter().enumerate() {
        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        let expected_block_number = n_origin_blocks + i + 2; // +1 for genesis, +1 for empty block
        assert_eq!(notification_block["block_number"], json!(expected_block_number));
        assert_eq!(notification_block["block_hash"], json!(expected_hash));
    }

    // Create additional blocks in the forked devnet after subscription and verify notifications
    let n_fork_blocks_after_subscription = 2;
    for i in 0..n_fork_blocks_after_subscription {
        let created_block_hash = fork_devnet.create_block().await.unwrap();

        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        // Verify the new block notification
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
        // Block number calculation: origin blocks + empty block + pre-subscription blocks + new
        // blocks
        let expected_block_number = n_origin_blocks + 1 + n_fork_blocks_before_subscription + i + 1;
        assert_eq!(notification_block["block_number"], json!(expected_block_number));
    }

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn test_fork_subscription_to_blocks_by_hash() {
    // Setup original devnet with some blocks
    let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

    // Create some blocks in origin
    let n_origin_blocks = 3;
    let mut origin_blocks = Vec::new();

    // Get genesis block
    let genesis_block = origin_devnet.get_latest_block_with_tx_hashes().await.unwrap();
    origin_blocks.push(genesis_block.clone());

    // Create blocks and store their info
    for _ in 0..n_origin_blocks {
        let block_hash = origin_devnet.create_block().await.unwrap();
        let block = origin_devnet
            .get_confirmed_block_with_tx_hashes(&BlockId::Hash(block_hash))
            .await
            .unwrap();
        origin_blocks.push(block);
    }

    // Fork the devnet
    let fork_devnet = origin_devnet.fork().await.unwrap();

    // Pick a block in the middle to start subscription from
    let start_block_idx = 1; // Start from the first non-genesis block
    let start_block = &origin_blocks[start_block_idx];

    // Subscribe to blocks starting from this specific block
    let (mut ws, _) = connect_async(fork_devnet.ws_url()).await.unwrap();
    let subscription_id =
        subscribe_new_heads(&mut ws, json!({ "block_id": BlockId::Hash(start_block.block_hash) }))
            .await
            .unwrap();

    // We should receive notifications for all blocks from start_block to latest
    for (i, block) in origin_blocks.iter().enumerate().skip(start_block_idx) {
        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(i));
        assert_eq!(notification_block["block_hash"], json!(block.block_hash));
    }

    // Devnet creates an empty block after forking, so we receive one more block notification
    let empty_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
    let empty_block_number = n_origin_blocks + 1;
    assert_eq!(empty_block["block_number"], json!(empty_block_number));

    // Create additional blocks in the forked devnet and verify notifications
    let n_fork_blocks = 2;
    for i in 0..n_fork_blocks {
        let created_block_hash = fork_devnet.create_block().await.unwrap();

        let notification_block = receive_block(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
        assert_eq!(notification_block["block_number"], json!(n_origin_blocks + i + 2)); // +1 for empty block
    }

    assert_no_notifications(&mut ws).await.unwrap();
}
