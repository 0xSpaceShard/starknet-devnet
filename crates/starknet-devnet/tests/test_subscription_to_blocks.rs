#![cfg(test)]
pub mod common;

mod websocket_subscription_support {
    use std::collections::HashMap;
    use std::time::Duration;

    use serde_json::json;
    use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
    use starknet_rs_providers::Provider;
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{assert_no_notifications, receive_rpc_via_ws, send_text_rpc_via_ws};

    async fn subscribe_new_heads(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        block_specifier: serde_json::Value,
    ) -> Result<i64, anyhow::Error> {
        let subscription_confirmation =
            send_text_rpc_via_ws(ws, "starknet_subscribeNewHeads", block_specifier).await?;
        subscription_confirmation["result"]
            .as_i64()
            .ok_or(anyhow::Error::msg("Subscription did not return a numeric ID"))
    }

    async fn unsubscribe(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        subscription_id: i64,
    ) -> Result<serde_json::Value, anyhow::Error> {
        send_text_rpc_via_ws(
            ws,
            "starknet_unsubscribe",
            json!({ "subscription_id": subscription_id }),
        )
        .await
    }

    #[tokio::test]
    async fn subscribe_to_new_block_heads_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_id = subscribe_new_heads(&mut ws, json!({})).await.unwrap();

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
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");

            assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), block_i);
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
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
        let subscription_id = subscribe_new_heads(
            &mut ws,
            json!({ "block_id": BlockId::Hash(genesis_block.block_hash)}),
        )
        .await
        .unwrap();

        let starting_block = 0;
        for block_i in starting_block..=n_blocks {
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");

            assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), block_i);
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
        }

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn subscription_to_pending_block_is_same_as_latest() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws_latest, _) = connect_async(devnet.ws_url()).await.unwrap();
        let (mut ws_pending, _) = connect_async(devnet.ws_url()).await.unwrap();

        // create two subscriptions: one to latest, one to pending
        let subscription_id_latest =
            subscribe_new_heads(&mut ws_latest, json!({ "block_id": "latest" })).await.unwrap();

        let subscription_id_pending =
            subscribe_new_heads(&mut ws_pending, json!({ "block_id": "pending" })).await.unwrap();

        assert_ne!(subscription_id_latest, subscription_id_pending);

        devnet.create_block().await.unwrap();

        // assert notification equality after taking subscription IDs out
        let mut notification_latest = receive_rpc_via_ws(&mut ws_latest).await.unwrap();
        assert_eq!(notification_latest["params"]["subscription_id"].take(), subscription_id_latest);
        assert_no_notifications(&mut ws_latest).await;

        let mut notification_pending = receive_rpc_via_ws(&mut ws_pending).await.unwrap();
        assert_eq!(
            notification_pending["params"]["subscription_id"].take(),
            subscription_id_pending
        );
        assert_no_notifications(&mut ws_pending).await;

        assert_eq!(notification_latest, notification_pending);
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
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
            assert_eq!(
                notification["params"]["result"]["block_hash"].as_str().unwrap(),
                created_block_hash.to_hex_string().as_str()
            );
            assert_eq!(
                notification["params"]["subscription_id"].as_i64().unwrap(),
                subscription_id
            );
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

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
        assert_eq!(
            notification["params"]["result"]["block_hash"].as_str().unwrap(),
            created_block_hash.to_hex_string().as_str()
        );

        assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), 1);
        assert_eq!(notification["params"]["subscription_id"].as_i64().unwrap(), subscription_id);
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

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
        assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), 1);
        assert_eq!(notification["params"]["subscription_id"].as_i64().unwrap(), subscription_id);

        // this assertion is skipped due to CI instability
        // assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn test_subscribing_to_non_existent_block() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        for block_id in [BlockId::Number(1), BlockId::Hash(Felt::ONE)] {
            let subscription_resp = send_text_rpc_via_ws(
                &mut ws,
                "starknet_subscribeNewHeads",
                json!({ "block_id": block_id }),
            )
            .await
            .unwrap();

            assert_eq!(
                subscription_resp,
                json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": 24, "message": "Block not found" } })
            );
        }
    }

    #[tokio::test]
    async fn test_aborted_blocks_not_subscribable() {
        let devnet_args = ["--state-archive-capacity", "full"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let new_block_hash = devnet.create_block().await.unwrap();
        devnet
            .send_custom_rpc(
                "devnet_abortBlocks",
                json!({ "starting_block_id": BlockId::Hash(new_block_hash) }),
            )
            .await
            .unwrap();

        let subscription_resp = send_text_rpc_via_ws(
            &mut ws,
            "starknet_subscribeNewHeads",
            json!({ "block_id": BlockId::Hash(new_block_hash) }),
        )
        .await
        .unwrap();

        assert_eq!(
            subscription_resp,
            json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": 24, "message": "Block not found" } })
        );
    }
}
