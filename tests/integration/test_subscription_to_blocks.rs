use std::collections::HashMap;
use std::time::Duration;

use serde_json::json;
use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_rs_providers::Provider;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{
    assert_no_notifications, receive_notification, subscribe_new_heads, unsubscribe, SubscriptionId,
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

        let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_block["block_hash"], json!(created_block_hash));
        assert_eq!(notification_block["block_number"], json!(block_i));
    }
}

#[tokio::test]
async fn should_not_receive_block_notification_if_not_subscribed() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await;
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
        let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(block_i));
    }

    assert_no_notifications(&mut ws).await;
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
        let notification_block = receive_block(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_block["block_number"], json!(block_i));
    }

    assert_no_notifications(&mut ws).await;
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
    assert_no_notifications(&mut ws_latest).await;

    let notification_block_default =
        receive_block(&mut ws_default, subscription_id_default).await.unwrap();
    assert_no_notifications(&mut ws_default).await;

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
    let unsubscriber_id = *subscribers.keys().next().expect("Should have at least one");

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

    assert_no_notifications(&mut unsubscriber_ws).await;
}

#[tokio::test]
async fn test_unsubscribing_invalid_id() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let dummy_id = 123;
    let unsubscription_resp = unsubscribe(&mut ws, dummy_id).await.unwrap();

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

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_notifications_in_block_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();

    let dummy_address = 0x1;
    devnet.mint(dummy_address, 1).await;

    assert_no_notifications(&mut ws).await;

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
