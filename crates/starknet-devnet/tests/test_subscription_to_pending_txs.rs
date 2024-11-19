#![cfg(test)]
pub mod common;

mod pending_transactions_subscription_support {
    use serde_json::json;
    use starknet_rs_core::types::{Felt, Transaction};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{assert_no_notifications, receive_rpc_via_ws, send_text_rpc_via_ws};

    async fn subscribe_pending_txs(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        with_details: bool,
        sender_address: &[Felt],
    ) -> Result<i64, anyhow::Error> {
        let params = json!({
            "transaction_details": with_details,
            "sender_address": sender_address,
        });

        let subscription_confirmation =
            send_text_rpc_via_ws(ws, "starknet_subscribePendingTransactions", params).await?;
        subscription_confirmation["result"]
            .as_i64()
            .ok_or(anyhow::Error::msg("Subscription did not return a numeric ID"))
    }

    #[tokio::test]
    async fn without_details_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_pending_txs(&mut ws, false, &[]).await.unwrap();

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
    async fn with_details_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_pending_txs(&mut ws, true, &[]).await.unwrap();

        let dummy_address = Felt::ONE;
        let amount = 123;
        let mint_hash = devnet.mint(dummy_address, amount).await;

        let mut notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        // extract the transaction from the response
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
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn with_empty_body_happy_path() {
        unimplemented!();
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
    async fn should_notify_if_txs_already_in_pending_block() {
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
    async fn should_not_notify_if_txs_already_in_latest_block() {
        unimplemented!();
    }

    // TODO add unsubscription tests here and in tx status tests
}
