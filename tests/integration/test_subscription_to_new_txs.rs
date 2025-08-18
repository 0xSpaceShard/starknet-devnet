use std::collections::HashMap;

use serde_json::json;
use starknet_core::constants::CHARGEABLE_ACCOUNT_ADDRESS;
use starknet_rs_accounts::{ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{
    DeclareTransactionV3, Felt, InvokeTransactionV3, Transaction, TransactionFinalityStatus,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::{
    FeeUnit, SubscriptionId, assert_no_notifications, declare_deploy_simple_contract,
    receive_notification, receive_rpc_via_ws, subscribe, unsubscribe,
};

async fn send_dummy_mint_tx(devnet: &BackgroundDevnet) -> Felt {
    devnet.mint(Felt::ONE, 123).await
}

async fn subscribe_new_txs(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    params: serde_json::Value,
) -> Result<SubscriptionId, anyhow::Error> {
    subscribe(ws, "starknet_subscribeNewTransactions", params).await
}

async fn receive_new_tx(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    expected_subscription_id: SubscriptionId,
) -> Result<serde_json::Value, anyhow::Error> {
    receive_notification(ws, "starknet_subscriptionNewTransaction", expected_subscription_id).await
}

#[tokio::test]
async fn should_not_notify_in_block_on_demand_mode_if_default_subscription_params() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // No notifications because default finality_status is ACCEPTED_ON_L2
    subscribe_new_txs(&mut ws, json!({})).await.unwrap();
    send_dummy_mint_tx(&devnet).await;
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_pre_confirmed_txs_with_block_generation_on_demand() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let finality_status = TransactionFinalityStatus::PreConfirmed;
    let subscription_params = json!({ "finality_status": [finality_status] });
    let subscription_id = subscribe_new_txs(&mut ws, subscription_params).await.unwrap();

    let tx_hash = send_dummy_mint_tx(&devnet).await;

    let mut notification_tx = receive_new_tx(&mut ws, subscription_id).await.unwrap();
    assert_eq!(notification_tx["finality_status"].take(), json!(finality_status));
    let extracted_tx: Transaction = serde_json::from_value(notification_tx).unwrap();
    assert_eq!(extracted_tx.transaction_hash(), &tx_hash);

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_accepted_on_l2_with_block_generation_on_tx() {
    let devnet_args = ["--block-generation-on", "transaction"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut explicit_ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut implicit_ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // One subscription is with explicit finality_status, the other with implicit/default
    let finality_status = TransactionFinalityStatus::AcceptedOnL2;
    let explicit_subscription_id =
        subscribe_new_txs(&mut explicit_ws, json!({ "finality_status": [finality_status] }))
            .await
            .unwrap();
    let implicit_subscription_id = subscribe_new_txs(&mut implicit_ws, json!({})).await.unwrap();

    let tx_hash = send_dummy_mint_tx(&devnet).await;

    for (mut ws, subscription_id) in
        [(explicit_ws, explicit_subscription_id), (implicit_ws, implicit_subscription_id)]
    {
        let mut notification_tx = receive_new_tx(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_tx["finality_status"].take(), json!(finality_status));
        let extracted_tx: Transaction = serde_json::from_value(notification_tx).unwrap();
        assert_eq!(extracted_tx.transaction_hash(), &tx_hash);
        assert_no_notifications(&mut ws).await;
    }
}

#[tokio::test]
async fn should_notify_for_multiple_subscribers_with_default_params() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let subscription_params = json!({});
    let mut subscribers = HashMap::new();
    for _ in 0..2 {
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id =
            subscribe_new_txs(&mut ws, subscription_params.clone()).await.unwrap();
        subscribers.insert(subscription_id, ws);
    }

    let tx_hash = send_dummy_mint_tx(&devnet).await;
    let finality_status = TransactionFinalityStatus::AcceptedOnL2;

    for (subscription_id, mut ws) in subscribers {
        let mut notification_tx = receive_new_tx(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_tx["finality_status"].take(), json!(finality_status));
        let extracted_tx: Transaction = serde_json::from_value(notification_tx).unwrap();
        assert_eq!(extracted_tx.transaction_hash(), &tx_hash);

        assert_no_notifications(&mut ws).await;
    }
}

#[tokio::test]
async fn should_stop_notifying_after_unsubscription() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let subscription_id = subscribe_new_txs(&mut ws, json!({})).await.unwrap();

    send_dummy_mint_tx(&devnet).await;
    receive_rpc_via_ws(&mut ws).await.unwrap();

    let unsubscription = unsubscribe(&mut ws, subscription_id).await.unwrap();
    assert_eq!(unsubscription, json!({ "jsonrpc": "2.0", "id": 0, "result": true }));

    send_dummy_mint_tx(&devnet).await;
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_for_filtered_address() {
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

    let subscription_id =
        subscribe_new_txs(&mut ws, json!({ "sender_address": [account_address] })).await.unwrap();

    // Send the actual txs
    let (class_hash, _) = declare_deploy_simple_contract(&predeployed_account).await.unwrap();

    // Assert received declaration notification
    let declaration_notification = receive_new_tx(&mut ws, subscription_id.clone()).await.unwrap();
    let declaration_tx: DeclareTransactionV3 =
        serde_json::from_value(declaration_notification).unwrap();
    assert_eq!(declaration_tx.class_hash, class_hash);
    assert_eq!(declaration_tx.nonce, Felt::ZERO);

    // Assert received deployment notification
    let deployment_notification = receive_new_tx(&mut ws, subscription_id).await.unwrap();
    let deployment_tx: InvokeTransactionV3 =
        serde_json::from_value(deployment_notification).unwrap();
    assert_eq!(deployment_tx.nonce, Felt::ONE);

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_not_notify_if_filtered_address_not_matched() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // dummy address
    subscribe_new_txs(&mut ws, json!({ "sender_address": ["0x1"] })).await.unwrap();

    send_dummy_mint_tx(&devnet).await;

    // nothing matched since minting is done via the Chargeable account
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_if_tx_by_filtered_address_already_in_pre_confirmed_block() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let mint_hash = send_dummy_mint_tx(&devnet).await;

    let finality_status = TransactionFinalityStatus::PreConfirmed;
    for subscription_request_body in [
        json!({ "finality_status": [finality_status] }),
        // Minting is done by the Chargeable account
        json!({ "finality_status": [finality_status], "sender_address": [CHARGEABLE_ACCOUNT_ADDRESS] }),
        json!({ "finality_status": [finality_status], "sender_address": [] }),
    ] {
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        let subscription_id = subscribe_new_txs(&mut ws, subscription_request_body).await.unwrap();

        let mut notification_tx = receive_new_tx(&mut ws, subscription_id).await.unwrap();
        assert_eq!(notification_tx["finality_status"].take(), json!(finality_status));
        let extracted_tx: Transaction = serde_json::from_value(notification_tx).unwrap();
        assert_eq!(extracted_tx.transaction_hash(), &mint_hash);

        assert_no_notifications(&mut ws).await;
    }
}

#[tokio::test]
async fn should_not_notify_if_tx_by_filtered_address_in_latest_block_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    send_dummy_mint_tx(&devnet).await;
    devnet.create_block().await.unwrap();

    // Minting is done by the Chargeable account
    let acceptable_address = CHARGEABLE_ACCOUNT_ADDRESS;
    subscribe_new_txs(
        &mut ws,
        json!({ "finality_status": ["PRE_CONFIRMED"], "sender_address": [acceptable_address] }),
    )
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
    send_dummy_mint_tx(&devnet).await;

    // Minting is done by the Chargeable account
    let acceptable_address = CHARGEABLE_ACCOUNT_ADDRESS;
    subscribe_new_txs(
        &mut ws,
        json!({ "finality_status": ["PRE_CONFIRMED"], "sender_address": [acceptable_address] }),
    )
    .await
    .unwrap();

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_not_notify_if_tx_already_in_latest_block_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    send_dummy_mint_tx(&devnet).await;
    devnet.create_block().await.unwrap();

    // Subscribe AFTER the tx and block creation.
    subscribe_new_txs(&mut ws, json!({ "finality_status": ["PRE_CONFIRMED"] })).await.unwrap();
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_not_notify_if_tx_already_in_latest_block_in_on_tx_mode() {
    let devnet_args = ["--block-generation-on", "transaction"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    send_dummy_mint_tx(&devnet).await;

    // Subscribe AFTER the tx and block creation.
    subscribe_new_txs(&mut ws, json!({ "finality_status": ["PRE_CONFIRMED"] })).await.unwrap();
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_not_notify_on_read_request_if_txs_in_pre_confirmed_block() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    subscribe_new_txs(&mut ws, json!({ "finality_status": ["PRE_CONFIRMED"] })).await.unwrap();

    send_dummy_mint_tx(&devnet).await;

    receive_rpc_via_ws(&mut ws).await.unwrap();

    // read request should have no impact
    let dummy_address = Felt::ONE;
    devnet.get_balance_latest(&dummy_address, FeeUnit::Wei).await.unwrap();

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_twice_if_subscribed_to_both_finality_statuses() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let finality_statuses =
        [TransactionFinalityStatus::PreConfirmed, TransactionFinalityStatus::AcceptedOnL2];
    let subscription_id =
        subscribe_new_txs(&mut ws, json!({ "finality_status": finality_statuses })).await.unwrap();

    let tx_hash = send_dummy_mint_tx(&devnet).await;

    for finality_status in finality_statuses {
        let mut notification_tx = receive_new_tx(&mut ws, subscription_id.clone()).await.unwrap();
        assert_eq!(notification_tx["finality_status"].take(), json!(finality_status));
        let extracted_tx: Transaction = serde_json::from_value(notification_tx).unwrap();
        assert_eq!(extracted_tx.transaction_hash(), &tx_hash);
        assert_no_notifications(&mut ws).await;
        devnet.create_block().await.unwrap();
    }

    assert_no_notifications(&mut ws).await;
}
