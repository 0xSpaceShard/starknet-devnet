use std::collections::{HashMap, HashSet};

use serde_json::json;
use starknet_rs_core::types::BlockId;
use tokio_tungstenite::connect_async;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{assert_no_notifications, receive_rpc_via_ws, subscribe, unsubscribe};

#[tokio::test]
async fn reorg_notification_for_all_subscriptions_except_pending_tx() {
    let devnet_args = ["--state-archive-capacity", "full"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    // create blocks for later abortion
    let starting_block_hash = devnet.create_block().await.unwrap();
    let ending_block_hash = devnet.create_block().await.unwrap();

    let mut notifiable_subscribers = HashMap::new();
    for (subscription_method, subscription_params) in [
        ("starknet_subscribeNewHeads", json!({})),
        ("starknet_subscribeTransactionStatus", json!({ "transaction_hash": "0x1" })),
        ("starknet_subscribeEvents", json!({})),
    ] {
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id =
            subscribe(&mut ws, subscription_method, subscription_params).await.unwrap();
        notifiable_subscribers.insert(subscription_id, ws);
    }

    let (mut unnotifiable_ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    subscribe(&mut unnotifiable_ws, "starknet_subscribePendingTransactions", json!({}))
        .await
        .unwrap();

    // assert that block-, tx_status- and event-subscribers got notified; unsubscribe them
    devnet.abort_blocks(&BlockId::Hash(starting_block_hash)).await.unwrap();
    for (subscription_id, ws) in notifiable_subscribers.iter_mut() {
        let notification = receive_rpc_via_ws(ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionReorg",
                "params": {
                    "result": {
                        "starting_block_hash": starting_block_hash,
                        "starting_block_number": 1,
                        "ending_block_hash": ending_block_hash,
                        "ending_block_number": 2,
                    },
                    "subscription_id": subscription_id,
                }
            })
        );
        unsubscribe(ws, *subscription_id).await.unwrap();
    }

    // now that all sockets are unsubscribed, abort a new block and assert no notifications
    let additional_block_hash = devnet.create_block().await.unwrap();
    devnet.abort_blocks(&BlockId::Hash(additional_block_hash)).await.unwrap();
    for (_, mut ws) in notifiable_subscribers {
        assert_no_notifications(&mut ws).await;
    }

    assert_no_notifications(&mut unnotifiable_ws).await;
}

#[tokio::test]
async fn socket_with_n_subscriptions_should_get_n_reorg_notifications() {
    let devnet_args = ["--state-archive-capacity", "full"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let created_block_hash = devnet.create_block().await.unwrap();

    // Create one socket with n subscriptions.
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let mut subscription_ids = vec![];
    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        let subscription_id = subscribe(&mut ws, subscription_method, json!({})).await.unwrap();
        subscription_ids.push(subscription_id);
    }

    // Trigger reorg.
    devnet.abort_blocks(&BlockId::Hash(created_block_hash)).await.unwrap();

    // Assert n reorg notifications received. The notifications only differ in subscription_id.
    let mut notification_ids = HashSet::new();
    for _ in subscription_ids.iter() {
        let mut notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        // Reorg notifications may be received in any order. To assert one reorg subscription
        // was received per subscription_id, we extract the IDs from notifications, store them
        // in a set, and later assert equality with the set of expected subscription IDs.
        let notification_id = notification["params"]["subscription_id"].take().as_u64().unwrap();
        notification_ids.insert(notification_id);

        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionReorg",
                "params": {
                    "result": {
                        "starting_block_hash": created_block_hash,
                        "starting_block_number": 1,
                        "ending_block_hash": created_block_hash,
                        "ending_block_number": 1,
                    },
                    "subscription_id": null,
                }
            })
        );
    }

    assert_eq!(notification_ids, HashSet::from_iter(subscription_ids));

    assert_no_notifications(&mut ws).await;
}
