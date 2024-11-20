#![cfg(test)]
pub mod common;

mod pending_transactions_subscription_support {
    use std::collections::HashMap;

    use serde_json::json;
    use starknet_core::constants::CHARGEABLE_ACCOUNT_ADDRESS;
    use starknet_rs_accounts::{ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::{DeclareTransaction, Felt, InvokeTransaction, Transaction};
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_no_notifications, declare_v3_deploy_v3,
        get_simple_contract_in_sierra_and_compiled_class_hash, receive_rpc_via_ws,
        send_text_rpc_via_ws, unsubscribe,
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

    /// Modifies the provided value by leaving a `null` in place of the returned transaction.
    fn extract_tx_from_notification(
        notification: &mut serde_json::Value,
    ) -> Result<Transaction, serde_json::Error> {
        let notification_result = notification["params"]["result"].take();
        serde_json::from_value(notification_result)
    }

    #[tokio::test]
    /// Both modes should notify of pending transactions, even though this never actually happens in
    /// transaction mode.
    async fn without_tx_details_happy_path() {
        for block_mode in ["transaction", "demand"] {
            let devnet_args = ["--block-generation-on", block_mode];
            let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

            let subscription_params = json!({ "transaction_details": false, "sender_address": [] });
            let subscription_id =
                subscribe_pending_txs(&mut ws, subscription_params).await.unwrap();

            let tx_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

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
    async fn without_tx_details_happy_path_multiple_subscribers() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let subscription_params = json!({ "transaction_details": false, "sender_address": [] });
        let mut subscribers = HashMap::new();
        for _ in 0..2 {
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
            let subscription_id =
                subscribe_pending_txs(&mut ws, subscription_params.clone()).await.unwrap();
            subscribers.insert(subscription_id, ws);
        }

        let tx_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

        for (subscription_id, mut ws) in subscribers {
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
    async fn with_tx_details_happy_path() {
        for block_mode in ["transaction", "demand"] {
            let devnet_args = ["--block-generation-on", block_mode];
            let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

            let subscription_params = json!({ "transaction_details": true, "sender_address": [] });
            let subscription_id =
                subscribe_pending_txs(&mut ws, subscription_params).await.unwrap();

            let mint_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

            let mut notification = receive_rpc_via_ws(&mut ws).await.unwrap();
            let notification_tx = extract_tx_from_notification(&mut notification).unwrap();

            // Just assert hash to simplify testing.
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

        let tx_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

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

        devnet.mint(Felt::ONE, 123).await; // dummy data

        receive_rpc_via_ws(&mut ws).await.unwrap();

        let unsubscription = unsubscribe(&mut ws, subscription_id).await.unwrap();
        assert_eq!(unsubscription, json!({ "jsonrpc": "2.0", "id": 0, "result": true }));

        devnet.mint(Felt::TWO, 456).await; // dummy data
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn with_tx_details_and_filtered_address_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let predeployed_account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let subscription_id = subscribe_pending_txs(
            &mut ws,
            json!({ "transaction_details": true, "sender_address": [account_address] }),
        )
        .await
        .unwrap();

        let (contract_class, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let (class_hash, _) = declare_v3_deploy_v3(
            &predeployed_account,
            contract_class.clone(),
            casm_hash,
            &[Felt::ONE], // dummy constructor
        )
        .await
        .unwrap();

        let mut declaration_notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        let declaration_tx = extract_tx_from_notification(&mut declaration_notification).unwrap();
        match declaration_tx {
            Transaction::Declare(DeclareTransaction::V3(tx)) => {
                assert_eq!(tx.class_hash, class_hash);
                assert_eq!(tx.nonce, Felt::ZERO);
            }
            other => panic!("Invalid tx: {other:?}"),
        };

        let mut deployment_notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        let deployment_tx = extract_tx_from_notification(&mut deployment_notification).unwrap();
        match deployment_tx {
            Transaction::Invoke(InvokeTransaction::V3(tx)) => {
                assert_eq!(tx.nonce, Felt::ONE);
            }
            other => panic!("Invalid tx: {other:?}"),
        };

        for notification in [declaration_notification, deployment_notification] {
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
        }

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_if_filtered_address_not_matched() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        // dummy address
        subscribe_pending_txs(&mut ws, json!({ "sender_address": ["0x1"] })).await.unwrap();

        devnet.mint(Felt::ONE, 123).await; // dummy data

        // nothing matched since minting is done via the Chargeable account
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_if_tx_by_filtered_address_already_in_pending_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let mint_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

        // Minting is done by the Chargeable account
        let acceptable_address = CHARGEABLE_ACCOUNT_ADDRESS;
        let subscription_id =
            subscribe_pending_txs(&mut ws, json!({ "sender_address": [acceptable_address] }))
                .await
                .unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionPendingTransactions",
                "params": {
                    "result": mint_hash,
                    "subscription_id": subscription_id,
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_if_tx_by_filtered_address_in_latest_block_in_on_demand_mode() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.mint(Felt::ONE, 123).await; // dummy data
        devnet.create_block().await.unwrap();

        // Minting is done by the Chargeable account
        let acceptable_address = CHARGEABLE_ACCOUNT_ADDRESS;
        subscribe_pending_txs(&mut ws, json!({ "sender_address": [acceptable_address] }))
            .await
            .unwrap();

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_if_tx_by_filtered_address_in_latest_block_in_on_tx_mode() {
        let devnet_args = ["--block-generation-on", "transaction"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        // Create tx and new block
        devnet.mint(Felt::ONE, 123).await; // dummy data

        // Minting is done by the Chargeable account
        let acceptable_address = CHARGEABLE_ACCOUNT_ADDRESS;
        subscribe_pending_txs(&mut ws, json!({ "sender_address": [acceptable_address] }))
            .await
            .unwrap();

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_if_tx_already_in_pending_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let tx_hash = devnet.mint(Felt::ONE, 123).await; // dummy data

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
    async fn should_not_notify_if_tx_already_in_latest_block_in_on_demand_mode() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.mint(Felt::ONE, 123).await; // dummy data
        devnet.create_block().await.unwrap();

        // Subscribe AFTER the tx and block creation.
        subscribe_pending_txs(&mut ws, json!({})).await.unwrap();
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_if_tx_already_in_latest_block_in_on_tx_mode() {
        let devnet_args = ["--block-generation-on", "transaction"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.mint(Felt::ONE, 123).await; // dummy data

        // Subscribe AFTER the tx and block creation.
        subscribe_pending_txs(&mut ws, json!({})).await.unwrap();
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_on_read_request_if_txs_in_pending_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        subscribe_pending_txs(&mut ws, json!({})).await.unwrap();

        let dummy_address = Felt::ONE;
        devnet.mint(dummy_address, 123).await; // dummy data

        receive_rpc_via_ws(&mut ws).await.unwrap();

        // read request should have no impact
        devnet.get_balance_latest(&dummy_address, FeeUnit::WEI).await.unwrap();

        assert_no_notifications(&mut ws).await;
    }
}
