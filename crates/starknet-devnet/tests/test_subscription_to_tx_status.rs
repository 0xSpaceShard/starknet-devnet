#![cfg(test)]
pub mod common;

mod tx_status_subscription_support {
    use serde_json::json;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
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

    /// Returns (address, amount, tx_hash), with tx_hash being the hash of the minting tx if it's
    /// the first thing done on a fresh Devnet using the returned `address` and `amount`.
    fn first_mint_data() -> (Felt, u128, Felt) {
        let expected_tx_hash = Felt::from_hex_unchecked(
            "0x2c13842a63d019b76805465c3cca99035ac82488856e7763e78427513021a13",
        );
        (Felt::ONE, 10, expected_tx_hash)
    }

    fn assert_successful_mint_notification(
        notification: serde_json::Value,
        tx_hash: Felt,
        subscription_id: i64,
    ) {
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionTransactionStatus",
                "params": {
                    "result": {
                        "transaction_hash": tx_hash,
                        "status": {
                            "finality_status": "ACCEPTED_ON_L2",
                            "failure_reason": null,
                            "execution_status": "SUCCEEDED",
                        },
                    },
                    "subscription_id": subscription_id,
                }
            })
        );
    }

    #[tokio::test]
    async fn subscribe_to_new_tx_status_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        let subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash, None).await.unwrap();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
    }

    #[tokio::test]
    async fn should_not_receive_notification_if_not_subscribed() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.mint(0x1, 1).await;
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_receive_notification_if_subscribed_to_another_tx() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        subscribe_tx_status(&mut ws, &Felt::ONE, None).await.unwrap();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

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

    #[tokio::test]
    async fn should_notify_only_when_tx_moved_from_pending_to_latest_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();
        let block_tag = BlockId::Tag(BlockTag::Latest);

        // should work if subscribing before sending the tx
        let (mut ws_before_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id_before =
            subscribe_tx_status(&mut ws_before_tx, &expected_tx_hash, Some(block_tag))
                .await
                .unwrap();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        assert_no_notifications(&mut ws_before_tx).await;

        // should work even if subscribing after the tx was sent
        let (mut ws_after_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id_after =
            subscribe_tx_status(&mut ws_after_tx, &expected_tx_hash, Some(block_tag))
                .await
                .unwrap();
        assert_no_notifications(&mut ws_after_tx).await;

        // move tx from pending to latest
        devnet.create_block().await.unwrap();

        for (subscription_id, mut ws) in
            [(subscription_id_before, ws_before_tx), (subscription_id_after, ws_after_tx)]
        {
            let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            assert_successful_mint_notification(notification, tx_hash, subscription_id);
            assert_no_notifications(&mut ws).await;
        }
    }

    #[tokio::test]
    async fn should_notify_pending_subscriber_if_tx_already_in_pending_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let (mut ws_pending, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_tx_status(
            &mut ws_pending,
            &expected_tx_hash,
            Some(BlockId::Tag(BlockTag::Pending)),
        )
        .await
        .unwrap();

        let notification = receive_rpc_via_ws(&mut ws_pending).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);

        // expect no notifications if subscribing to latest block and latest not yet mined
        let (mut ws_latest, _) = connect_async(devnet.ws_url()).await.unwrap();
        subscribe_tx_status(
            &mut ws_latest,
            &expected_tx_hash,
            Some(BlockId::Tag(BlockTag::Latest)),
        )
        .await
        .unwrap();
        assert_no_notifications(&mut ws_latest).await;
    }

    #[tokio::test]
    async fn should_notify_if_tx_already_in_latest_block_and_subscribed_to_latest() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);
        devnet.create_block().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id =
            subscribe_tx_status(&mut ws, &expected_tx_hash, Some(BlockId::Tag(BlockTag::Latest)))
                .await
                .unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
    }

    #[tokio::test]
    async fn should_not_notify_if_tx_already_in_latest_block_and_subscribed_to_pending() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);
        devnet.create_block().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id =
            subscribe_tx_status(&mut ws, &expected_tx_hash, Some(BlockId::Tag(BlockTag::Latest)))
                .await
                .unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
    }

    #[tokio::test]
    async fn should_never_notify_if_block_id_newer_than_tx_addition() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let newer_block_hash = devnet.create_block().await.unwrap();
        subscribe_tx_status(&mut ws, &tx_hash, Some(BlockId::Hash(newer_block_hash)))
            .await
            .unwrap();

        assert_no_notifications(&mut ws).await;

        // regardless of how many blocks are created, no notifications are expected
        devnet.create_block().await.unwrap();
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_when_tx_in_pending_if_legal_numeric_block_id() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();
        let subscription_id =
            subscribe_tx_status(&mut ws, &expected_tx_hash, Some(BlockId::Number(0)))
                .await
                .unwrap();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);

        // should only have notified for adding the tx in pending, not now when latest is created
        devnet.create_block().await.unwrap();
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_when_tx_in_latest_if_legal_numeric_block_id() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (address, mint_amount, expected_tx_hash) = first_mint_data();
        let subscription_id =
            subscribe_tx_status(&mut ws, &expected_tx_hash, Some(BlockId::Number(0)))
                .await
                .unwrap();

        let tx_hash = devnet.mint(address, mint_amount).await;
        assert_eq!(tx_hash, expected_tx_hash);

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
    }

    #[tokio::test]
    async fn should_return_error_for_invalid_block_id() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        for block_id in [BlockId::Number(1), BlockId::Hash(Felt::ONE)] {
            let resp = send_text_rpc_via_ws(
                &mut ws,
                "starknet_subscribeTransactionStatus",
                json!({ "transaction_hash": Felt::ONE, "block": block_id }),
            )
            .await
            .unwrap();

            let expected_error = json!({ "code": 24, "message": "Block not found" });
            assert_eq!(resp, json!({ "jsonrpc": "2.0", "id": 0, "error": expected_error }));
        }
    }
}
