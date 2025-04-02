use serde_json::json;
use starknet_rs_core::types::{BlockId, Felt};
use starknet_rs_providers::Provider;
use tokio_tungstenite::connect_async;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::MAINNET_URL;
use crate::common::utils::{assert_no_notifications, send_text_rpc_via_ws};

const MAXIMUM_BLOCKS_BEHIND: u64 = 1024;

fn block_not_found_error() -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": 24, "message": "Block not found" } })
}

fn call_on_pending_error() -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": -32602, "message": "Subscription block cannot be 'pending'" }})
}

#[tokio::test]
async fn test_subscribing_to_non_existent_block() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // Cartesian product: subscription_method x invalid_block_id
    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        for block_id in [BlockId::Number(1), BlockId::Hash(Felt::ONE)] {
            let subscription_resp =
                send_text_rpc_via_ws(&mut ws, subscription_method, json!({ "block_id": block_id }))
                    .await
                    .unwrap();

            assert_eq!(subscription_resp, block_not_found_error())
        }
    }

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_aborted_blocks_not_subscribable() {
    let devnet_args = ["--state-archive-capacity", "full"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let created_block_hash = devnet.create_block().await.unwrap();
    devnet.abort_blocks(&BlockId::Hash(created_block_hash)).await.unwrap();

    // Cartesian product: subscription_method x invalid_block_id
    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        for block_id in [BlockId::Number(1), BlockId::Hash(created_block_hash)] {
            let subscription_resp =
                send_text_rpc_via_ws(&mut ws, subscription_method, json!({ "block_id": block_id }))
                    .await
                    .unwrap();

            assert_eq!(subscription_resp, block_not_found_error())
        }
    }

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_pending_block_not_allowed_in_block_and_event_subscription() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        let subscription_resp =
            send_text_rpc_via_ws(&mut ws, subscription_method, json!({ "block_id": "pending" }))
                .await
                .unwrap();

        assert_eq!(subscription_resp, call_on_pending_error(), "Method: {subscription_method}");
    }
}

#[tokio::test]
async fn test_subscribing_to_too_many_blocks_behind() {
    let forked_devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--fork-network", MAINNET_URL])
            .await
            .unwrap();

    let (mut ws, _) = connect_async(forked_devnet.ws_url()).await.unwrap();

    let latest_block_number = forked_devnet.json_rpc_client.block_number().await.unwrap();

    let too_old_block_id =
        json!({"block_id": { "block_number": latest_block_number - MAXIMUM_BLOCKS_BEHIND - 1 }});

    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        let subscription_resp =
            send_text_rpc_via_ws(&mut ws, subscription_method, too_old_block_id.clone())
                .await
                .unwrap();

        assert_eq!(
            subscription_resp,
            json!({
                "jsonrpc": "2.0",
                "id": 0,
                "error": {
                    "code": 40,
                    "message": "Too many blocks behind",
                },
            }),
            "Method: {subscription_method}"
        );
    }
}

#[tokio::test]
async fn test_subscribing_to_maximum_blocks_behind() {
    let forked_devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--fork-network", MAINNET_URL])
            .await
            .unwrap();

    let (mut ws, _) = connect_async(forked_devnet.ws_url()).await.unwrap();

    let latest_block_number = forked_devnet.json_rpc_client.block_number().await.unwrap();

    let maximum_block_id =
        json!({"block_id": { "block_number": latest_block_number - MAXIMUM_BLOCKS_BEHIND }});

    for subscription_method in ["starknet_subscribeNewHeads", "starknet_subscribeEvents"] {
        let _subscription_id =
            send_text_rpc_via_ws(&mut ws, subscription_method, maximum_block_id.clone())
                .await
                .unwrap();
    }
}

// TODO consider moving these tests to their respective test files. Having error cases inside a
// separate file is dubious.
