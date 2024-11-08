#![cfg(test)]
pub mod common;

mod tx_status_subscription_support {
    use serde_json::json;
    use starknet_rs_core::types::{BlockId, Felt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        assert_no_notifications, receive_rpc_via_ws, send_text_rpc_via_ws, subscribe_new_heads,
    };

    async fn subscribe_tx_status(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        tx_hash: &Felt,
        block_id: Option<BlockId>,
    ) -> Result<i64, anyhow::Error> {
        let mut params = json!({ "transaction_hash": tx_hash });

        if let Some(block_id) = block_id {
            params["block"] = json!(block_id);
        }

        let subscription_confirmation =
            send_text_rpc_via_ws(ws, "starknet_subscribeTransactionStatus", params).await?;
        subscription_confirmation["result"]
            .as_i64()
            .ok_or(anyhow::Error::msg("Subscription did not return a numeric ID"))
    }

    #[tokio::test]
    async fn subscribe_to_new_tx_status_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let address = Felt::ONE;
        let amount = 10;

        // expected minting hash, precalculated based on address and amount
        let expected_tx_hash = Felt::from_hex_unchecked(
            "0x2c13842a63d019b76805465c3cca99035ac82488856e7763e78427513021a13",
        );

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let _subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash, None).await.unwrap();

        let tx_hash = devnet.mint(address, amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(notification["method"], "starknet_subscriptionTransactionStatus");
        assert_eq!(
            notification["params"]["result"],
            json!({
                "transaction_hash": expected_tx_hash,
                "status": "ACCEPTED_ON_L2",
            })
        );
    }

    #[tokio::test]
    async fn should_not_receive_notification_if_not_subscribed() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.mint(0x1, 1).await;
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_receive_tx_notification_if_subscribed_to_blocks() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        subscribe_new_heads(&mut ws, json!({})).await.unwrap();

        devnet.mint(0x1, 1).await;

        // there should only be a single new block notification since minting is a block-adding tx
        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_receive_block_notification_if_subscribed_to_tx() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        subscribe_tx_status(&mut ws, &Felt::ONE, None).await.unwrap();

        devnet.create_block().await.unwrap();
        assert_no_notifications(&mut ws).await;
    }
}
