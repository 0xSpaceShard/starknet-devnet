#![cfg(test)]
pub mod common;

mod event_subscription_support {
    use serde::Serialize;
    use serde_json::json;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::{BlockId, Call, Felt, InvokeTransactionResult};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::JsonRpcClient;
    use starknet_rs_signers::LocalWallet;
    use starknet_types::contract_address::ContractAddress;
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_no_notifications, declare_v3_deploy_v3,
        get_events_contract_in_sierra_and_compiled_class_hash, receive_rpc_via_ws,
        send_text_rpc_via_ws, unsubscribe,
    };

    async fn subscribe_events(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        params: serde_json::Value,
    ) -> Result<i64, anyhow::Error> {
        let subscription_confirmation =
            send_text_rpc_via_ws(ws, "starknet_subscribeEvents", params).await?;
        subscription_confirmation["result"]
            .as_i64()
            .ok_or(anyhow::Error::msg("Subscription did not return a numeric ID"))
    }

    #[derive(Serialize)]
    struct EventParams {
        from_address: Option<ContractAddress>,
        keys: Option<Vec<Vec<Felt>>>,
        block: Option<BlockId>,
    }

    async fn get_single_owner_account(
        devnet: &BackgroundDevnet,
    ) -> SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet> {
        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        )
    }

    /// Returns deployment address.
    async fn deploy_events_contract(
        account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    ) -> Felt {
        let (sierra, casm_hash) = get_events_contract_in_sierra_and_compiled_class_hash();

        let (_, address) = declare_v3_deploy_v3(account, sierra, casm_hash, &[]).await.unwrap();
        address
    }

    async fn emit_static_event(
        account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
        contract_address: Felt,
    ) -> Result<InvokeTransactionResult, anyhow::Error> {
        account
            .execute_v3(vec![Call {
                to: contract_address,
                selector: get_selector_from_name("emit_event").unwrap(),
                calldata: vec![Felt::ZERO], // what kind of event to emit
            }])
            .send()
            .await
            .map_err(|e| anyhow::Error::msg(e.to_string()))
    }

    #[tokio::test]
    async fn event_subscription_with_no_params_until_unsubscription() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_id = subscribe_events(&mut ws, json!({})).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        let _invocation = emit_static_event(&account, contract_address).await.unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;

        unsubscribe(&mut ws, subscription_id).await.unwrap();

        emit_static_event(&account, contract_address).await.unwrap();
        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_only_from_filtered_address() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        let subscription_params = json!({ "from_address": contract_address });
        let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

        emit_static_event(&account, contract_address).await.unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_of_new_events_only_from_filtered_key_and_address() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        let subscription_params = json!({ "from_address": contract_address, "keys": [[]] });
        let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

        emit_static_event(&account, contract_address).await.unwrap();

        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_if_already_in_latest_block_in_on_tx_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;
        emit_static_event(&account, contract_address).await.unwrap();

        let subscription_id = subscribe_events(&mut ws, json!({})).await.unwrap();
        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_if_already_in_latest_block_in_on_demand_mode() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;
        // to have declare+deploy and invoke in two separate blocks
        devnet.create_block().await.unwrap();

        emit_static_event(&account, contract_address).await.unwrap();
        devnet.create_block().await.unwrap();

        let subscription_id = subscribe_events(&mut ws, json!({})).await.unwrap();
        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_only_when_moved_from_pending_to_latest_block() {
        let devnet_args = ["--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        // to have declare+deploy and invoke in two separate blocks
        devnet.create_block().await.unwrap();

        let subscription_params = json!({ "from_address": contract_address });
        let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

        // only receive event once pending->latest
        emit_static_event(&account, contract_address).await.unwrap();
        assert_no_notifications(&mut ws).await;

        devnet.create_block().await.unwrap();
        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(
            notification,
            json!({
                "jsonrpc": "2.0",
                "method": "starknet_subscriptionEvents",
                "params": {
                    "subscription_id": subscription_id,
                    "result": {},
                }
            })
        );

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_of_events_in_old_blocks_with_no_filters() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        emit_static_event(&account, contract_address).await.unwrap();

        // The declaration happens at block_number=1
        subscribe_events(&mut ws, json!({ "block": BlockId::Number(1) })).await.unwrap();

        let _declaration_notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        let _deployment_notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        let invocation_notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(invocation_notification, json!({}));

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_of_events_in_old_blocks_with_address_filter() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        emit_static_event(&account, contract_address).await.unwrap();

        // The declaration happens at block_number=1, but only invocation should be notified of
        subscribe_events(
            &mut ws,
            json!({ "block": BlockId::Number(1), "from_address": contract_address }),
        )
        .await
        .unwrap();

        let invocation_notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(invocation_notification, json!({}));

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_notify_of_old_and_new_events_with_address_and_key_filter() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        emit_static_event(&account, contract_address).await.unwrap();

        // The declaration happens at block_number=1, but only invocation should be notified of
        subscribe_events(
            &mut ws,
            json!({ "block": BlockId::Number(1), "from_address": contract_address, "keys": [[]] }),
        )
        .await
        .unwrap();

        let invocation_notification = receive_rpc_via_ws(&mut ws).await.unwrap();

        assert_eq!(invocation_notification, json!({}));

        assert_no_notifications(&mut ws).await;
    }

    #[tokio::test]
    async fn should_not_notify_of_events_in_too_old_blocks() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let account = get_single_owner_account(&devnet).await;
        let contract_address = deploy_events_contract(&account).await;

        emit_static_event(&account, contract_address).await.unwrap();

        let last_block_hash = devnet.create_block().await.unwrap();

        subscribe_events(&mut ws, json!({ "block": BlockId::Hash(last_block_hash) }))
            .await
            .unwrap();

        assert_no_notifications(&mut ws).await;
    }
}
