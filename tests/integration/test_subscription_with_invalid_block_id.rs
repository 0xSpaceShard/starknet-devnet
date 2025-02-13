use serde_json::json;
use starknet_rs_core::types::{BlockId, Felt};
use tokio_tungstenite::connect_async;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{assert_no_notifications, send_text_rpc_via_ws};

fn block_not_found_error() -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": 24, "message": "Block not found" } })
}

fn call_on_pending_error() -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": 69, "message": "This method does not support being called on the pending block" } })
}

#[tokio::test]
async fn test_subscribing_to_non_existent_block() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // Cartesian product: (subscription_method, subscription_params) x invalid_block_id
    for (subscription_method, mut subscription_params) in [
        ("starknet_subscribeNewHeads", json!({})),
        ("starknet_subscribeTransactionStatus", json!({ "transaction_hash": "0x1" })),
        ("starknet_subscribeEvents", json!({})),
    ] {
        for block_id in [BlockId::Number(1), BlockId::Hash(Felt::ONE)] {
            subscription_params["block_id"] = json!(block_id);
            let subscription_resp =
                send_text_rpc_via_ws(&mut ws, subscription_method, subscription_params.clone())
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

    // Cartesian product: (subscription_method, subscription_params) x invalid_block_id
    for (subscription_method, mut subscription_params) in [
        ("starknet_subscribeNewHeads", json!({})),
        ("starknet_subscribeTransactionStatus", json!({ "transaction_hash": "0x1" })),
        ("starknet_subscribeEvents", json!({})),
    ] {
        for block_id in [BlockId::Number(1), BlockId::Hash(created_block_hash)] {
            subscription_params["block_id"] = json!(block_id);
            let subscription_resp =
                send_text_rpc_via_ws(&mut ws, subscription_method, subscription_params.clone())
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
