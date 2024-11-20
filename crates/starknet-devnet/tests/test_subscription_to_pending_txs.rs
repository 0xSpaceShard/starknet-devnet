#![cfg(test)]
pub mod common;

mod pending_transactions_subscription_support {
    use serde_json::json;
    use starknet_rs_core::types::{Felt, Transaction};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        assert_no_notifications, receive_rpc_via_ws, send_text_rpc_via_ws, unsubscribe,
    };

    async fn subscribe_pending_txs(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        params: serde_json::Value,
    ) -> Result<i64, anyhow::Error> {
        let subscription_confirmation =
            send_text_rpc_via_ws(ws, "starknet_subscribePendingTransactions", params).await?;
        subscription_confirmation["result"]
            .as_i64()
            .ok_or(anyhow::Error::msg("Subscription did not return a numeric ID"))
    }

    #[tokio::test]
    /// Both modes should notify of pending transactions, even though this never actually happens in
    /// transaction mode.
    async fn without_details_happy_path() {
        for block_mode in ["transaction", "demand"] {
            let devnet_args = ["--block-generation-on", block_mode];
            let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

            let subscription_params = json!({ "transaction_details": false, "sender_address": [] });
            let subscription_id =
                subscribe_pending_txs(&mut ws, subscription_params).await.unwrap();

            let dummy_address = Felt::ONE;
            let amount = 123;
            let tx_hash = devnet.mint(dummy_address, amount).await;

            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_eq!(
                notification,
                json!({
                    "jsonrpc": "2.0",
                    "method": "starknet_subscriptionPendingTransactions",
                    "params": {
                        "result": tx_hash,
                        "subscription_id": subscription_id,
                    }
                })
            );

            assert_no_notifications(&mut ws).await;
        }
    }

    #[tokio::test]
    /// Both modes should notify of pending transactions, even though this never actually happens in
    /// transaction mode.
    async fn with_details_happy_path() {
        for block_mode in ["transaction", "demand"] {
            let devnet_args = ["--block-generation-on", block_mode];
            let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

            let subscription_params = json!({ "transaction_details": true, "sender_address": [] });
            let subscription_id =
                subscribe_pending_txs(&mut ws, subscription_params).await.unwrap();

            let dummy_address = Felt::ONE;
            let amount = 123;
            let mint_hash = devnet.mint(dummy_address, amount).await;

            let mut notification = receive_rpc_via_ws(&mut ws).await.unwrap();

            // Extract the transaction from the response; assert hash to simplify testing.
            let notification_result = notification["params"]["result"].take();
            let notification_tx: Transaction = serde_json::from_value(notification_result).unwrap();
            assert_eq!(notification_tx.transaction_hash(), &mint_hash);

            assert_eq!(
                notification,
                json!({
                    "jsonrpc": "2.0",
                    "method": "starknet_subscriptionPendingTransactions",
                    "params": {
                        "subscription_id": subscription_id,
                        "result": null,
                    }
                })
            );

            assert_no_notifications(&mut ws).await;
        }
    }

    #[tokio::test]
    async fn with_empty_body_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_id = subscribe_pending_txs(&mut ws, json!({})).await.unwrap();

        let dummy_address = Felt::ONE;
        let amount = 123;
        let tx_hash = devnet.mint(dummy_address, amount).await;

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionPendingTransactions",
                "params": {
                    "result": tx_hash,
                    "subscription_id": subscription_id,
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_stop_notifying_after_unsubscription() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_id = subscribe_pending_txs(&mut ws, json!({})).await.unwrap();

        let dummy_address = Felt::ONE;
        let amount = 123;
        devnet.mint(dummy_address, amount).await;

        receive_rpc_via_ws(&mut ws).await.unwrap();

        let unsubscription = unsubscribe(&mut ws, subscription_id).await.unwrap();
        assert_eq!(unsubscription, json!({ "jsonrpc": "2.0", "id": 0, "result": true }));

        devnet.mint(dummy_address, amount).await;
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn with_details_and_filtered_address_happy_path() {
        unimplemented!();
    }

    #[tokio::test]
    async fn should_not_notify_if_filtered_address_not_matched() {
        unimplemented!();
    }

    #[tokio::test]
    async fn should_notify_if_txs_by_filtered_address_already_in_pending_block() {
        unimplemented!();
    }

    #[tokio::test]
    async fn should_not_notify_if_txs_by_filtered_address_already_in_latest_block() {
        unimplemented!();
    }

    #[tokio::test]
    async fn should_notify_if_txs_already_in_pending_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let dummy_address = Felt::ONE;
        let amount = 123;
        let tx_hash = devnet.mint(dummy_address, amount).await;

        // Subscribe AFTER the tx.
        let subscription_id = subscribe_pending_txs(&mut ws, json!({})).await.unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionPendingTransactions",
                "params": {
                    "result": tx_hash,
                    "subscription_id": subscription_id,
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_if_txs_already_in_latest_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let dummy_address = Felt::ONE;
        let amount = 123;
        devnet.mint(dummy_address, amount).await;

        devnet.create_block().await.unwrap();

        // Subscribe AFTER the tx and block creation.
        subscribe_pending_txs(&mut ws, json!({})).await.unwrap();
        assert_no_notifications(&mut ws).await;
    }
}
