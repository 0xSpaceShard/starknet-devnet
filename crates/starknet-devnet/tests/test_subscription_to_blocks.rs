#![cfg(test)]
pub mod common;

mod websocket_subscription_support {
    use std::collections::HashMap;

    use serde_json::json;
    use server::test_utils::assert_contains;
    use tokio_tungstenite::connect_async;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{receive_rpc_via_ws, send_text_rpc_via_ws};

    #[tokio::test]
    async fn subscribe_to_new_block_heads_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_confirmation =
            send_text_rpc_via_ws(&mut ws, "starknet_subscribeNewHeads", json!({})).await.unwrap();
        let subscription_id = subscription_confirmation["result"].as_i64().unwrap();

        // test with multiple blocks created, number 0 was origin, so we start at 1
        for block_i in 1..=2 {
            let created_block_hash = devnet.create_block().await.unwrap();

            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
            assert_eq!(
                notification["params"]["result"]["block_hash"].as_str().unwrap(),
                created_block_hash.to_hex_string().as_str()
            );

            assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), block_i);
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
        }
    }

    #[tokio::test]
    async fn should_not_receive_block_notification_if_not_subscribed() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.create_block().await.unwrap();
        let read_err = receive_rpc_via_ws(&mut ws).await.unwrap_err();
        assert_contains(read_err.to_string().as_str(), "deadline has elapsed");
    }

    #[tokio::test]
    async fn multiple_block_subscribers_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let n_subscribers = 5;

        let mut subscribers = HashMap::new();
        for _ in 0..n_subscribers {
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
            let subscription_confirmation =
                send_text_rpc_via_ws(&mut ws, "starknet_subscribeNewHeads", json!({}))
                    .await
                    .unwrap();

            let subscription_id = subscription_confirmation["result"].as_i64().unwrap();
            subscribers.insert(subscription_id, ws);
        }

        let created_block_hash = devnet.create_block().await.unwrap();

        for (subscription_id, mut ws) in subscribers {
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
            assert_eq!(
                notification["params"]["result"]["block_hash"].as_str().unwrap(),
                created_block_hash.to_hex_string().as_str()
            );

            assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), 1);
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
        }
    }

    #[tokio::test]
    async fn subscription_to_an_older_block_should_notify_of_all_blocks_up_to_latest() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let n_blocks = 5;
        for _ in 0..n_blocks {
            devnet.create_block().await.unwrap();
        }

        // request notifications for all blocks starting with genesis
        let starting_block = 0;
        let subscription_confirmation = send_text_rpc_via_ws(
            &mut ws,
            "starknet_subscribeNewHeads",
            json!({ "block_id": { "block_number": starting_block } }),
        )
        .await
        .unwrap();
        let subscription_id = subscription_confirmation["result"].as_i64().unwrap();

        for block_i in starting_block..=n_blocks {
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");

            assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), block_i);
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
        }

        // assert no more messages to receive
        let read_err = receive_rpc_via_ws(&mut ws).await.unwrap_err();
        assert_contains(read_err.to_string().as_str(), "deadline has elapsed");
    }
}
