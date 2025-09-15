use serde_json::json;
use starknet_rs_core::types::{BlockId, Felt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{
    SubscriptionId, assert_no_notifications, receive_rpc_via_ws, subscribe, subscribe_new_heads,
    unsubscribe,
};

async fn subscribe_tx_status(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    tx_hash: &Felt,
) -> Result<SubscriptionId, anyhow::Error> {
    subscribe(ws, "starknet_subscribeTransactionStatus", json!({ "transaction_hash": tx_hash }))
        .await
}

/// Returns (address, amount, tx_hash), with tx_hash being the hash of the minting tx if it's
/// the first thing done on a fresh Devnet using the returned `address` and `amount`.
fn first_mint_data() -> (Felt, u128, Felt) {
    let expected_tx_hash = Felt::from_hex_unchecked(
        "0x40c9808b4812c58bcd71323527ead6a3f74c802fd3ded08b9653f2e19a67f11",
    );
    (Felt::ONE, 10, expected_tx_hash)
}

fn assert_mint_notification_succeeded(
    notification: serde_json::Value,
    tx_hash: Felt,
    subscription_id: SubscriptionId,
    expected_finality_status: &str,
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
                        "finality_status": expected_finality_status,
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

    let subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_mint_notification_succeeded(notification, tx_hash, subscription_id, "ACCEPTED_ON_L2");
}

#[tokio::test]
async fn should_stop_notifying_after_unsubscription() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();

    // subscribe and immediately unsubscribe
    let subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash).await.unwrap();
    let unsubscription = unsubscribe(&mut ws, subscription_id).await.unwrap();
    assert_eq!(unsubscription, json!({ "jsonrpc": "2.0", "id": 0, "result": true }));

    devnet.mint(address, mint_amount).await;
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_not_receive_notification_if_not_subscribed() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    devnet.mint(0x1, 1).await;
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_not_receive_notification_if_subscribed_to_another_tx() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let dummy_tx_hash = Felt::ONE;
    subscribe_tx_status(&mut ws, &dummy_tx_hash).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();
    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);
    assert_ne!(tx_hash, dummy_tx_hash);

    assert_no_notifications(&mut ws).await.unwrap();
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
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_not_receive_block_notification_if_subscribed_to_tx() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    subscribe_tx_status(&mut ws, &Felt::ONE).await.unwrap();

    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await.unwrap();
}

async fn should_notify_if_subscribed_before_and_after_tx(
    devnet: &BackgroundDevnet,
    ws_before_tx: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ws_after_tx: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    expected_finality_status: &str,
) -> (Felt, String, String) {
    let (address, mint_amount, expected_tx_hash) = first_mint_data();

    // should work if subscribing before sending the tx
    let subscription_id_before =
        subscribe_tx_status(ws_before_tx, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    {
        let notification = receive_rpc_via_ws(ws_before_tx).await.unwrap();
        assert_mint_notification_succeeded(
            notification,
            tx_hash,
            subscription_id_before.clone(),
            expected_finality_status,
        );
        assert_no_notifications(ws_before_tx).await.unwrap();
    }

    // should work even if subscribing after the tx was sent
    let subscription_id_after = subscribe_tx_status(ws_after_tx, &expected_tx_hash).await.unwrap();
    {
        let notification = receive_rpc_via_ws(ws_after_tx).await.unwrap();
        assert_mint_notification_succeeded(
            notification,
            tx_hash,
            subscription_id_after.clone(),
            expected_finality_status,
        );
        assert_no_notifications(ws_after_tx).await.unwrap();
    }

    (tx_hash, subscription_id_before, subscription_id_after)
}

#[tokio::test]
async fn should_notify_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws_before_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut ws_after_tx, _) = connect_async(devnet.ws_url()).await.unwrap();

    let (mint_tx_hash, subscription_id_before, subscription_id_after) =
        should_notify_if_subscribed_before_and_after_tx(
            &devnet,
            &mut ws_before_tx,
            &mut ws_after_tx,
            "PRE_CONFIRMED",
        )
        .await;

    // Creating a new block should make txs go: PRE_CONFIRMED->ACCEPTED_ON_L2
    devnet.create_block().await.unwrap();

    for (mut ws, subscription_id) in
        [(ws_before_tx, subscription_id_before), (ws_after_tx, subscription_id_after)]
    {
        let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
        assert_mint_notification_succeeded(
            notification,
            mint_tx_hash,
            subscription_id,
            "ACCEPTED_ON_L2",
        );
        assert_no_notifications(&mut ws).await.unwrap();
    }
}

#[tokio::test]
async fn should_notify_only_once_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_mint_notification_succeeded(notification, tx_hash, subscription_id, "PRE_CONFIRMED");
    assert_no_notifications(&mut ws).await.unwrap();

    let another_tx_hash = devnet.mint(address, mint_amount).await;
    assert_ne!(another_tx_hash, tx_hash);
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_in_on_transaction_mode() {
    let devnet_args = ["--block-generation-on", "transaction"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws_before_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut ws_after_tx, _) = connect_async(devnet.ws_url()).await.unwrap();

    should_notify_if_subscribed_before_and_after_tx(
        &devnet,
        &mut ws_before_tx,
        &mut ws_after_tx,
        "ACCEPTED_ON_L2",
    )
    .await;

    // Expect no new notifications on creating a new empty block
    devnet.create_block().await.unwrap();

    assert_no_notifications(&mut ws_before_tx).await.unwrap();
    assert_no_notifications(&mut ws_after_tx).await.unwrap();
}

#[tokio::test]
async fn should_notify_if_already_in_latest() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // created tx
    let (address, mint_amount, _) = first_mint_data();
    let tx_hash = devnet.mint(address, mint_amount).await;

    let subscription_id = subscribe_tx_status(&mut ws, &tx_hash).await.unwrap();

    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_mint_notification_succeeded(notification, tx_hash, subscription_id, "ACCEPTED_ON_L2");

    devnet.mint(address, mint_amount).await;
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_if_already_in_an_old_block() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    // created tx
    let (address, mint_amount, _) = first_mint_data();
    let tx_hash = devnet.mint(address, mint_amount).await;

    // push back the block with the tx
    devnet.create_block().await.unwrap();
    devnet.create_block().await.unwrap();

    let subscription_id = subscribe_tx_status(&mut ws, &tx_hash).await.unwrap();

    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_mint_notification_succeeded(notification, tx_hash, subscription_id, "ACCEPTED_ON_L2");

    devnet.mint(address, mint_amount).await;
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_not_notify_of_status_change_when_block_aborted() {
    let devnet_args = ["--state-archive-capacity", "full"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let (address, amount, _) = first_mint_data();
    let tx_hash = devnet.mint(address, amount).await;
    let subscription_id = subscribe_tx_status(&mut ws, &tx_hash).await.unwrap();

    // as expected, the actual tx accepted notification is first
    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_mint_notification_succeeded(notification, tx_hash, subscription_id, "ACCEPTED_ON_L2");

    devnet.abort_blocks(&BlockId::Number(1)).await.unwrap();

    // only expect reorg subscription
    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_eq!(notification["method"], "starknet_subscriptionReorg");
    assert_no_notifications(&mut ws).await.unwrap();
}
