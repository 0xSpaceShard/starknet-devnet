use serde_json::json;
use starknet_rs_core::types::{BlockId, Felt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{
    assert_no_notifications, receive_rpc_via_ws, subscribe, subscribe_new_heads, unsubscribe,
    SubscriptionId,
};

async fn subscribe_tx_status(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    tx_hash: &Felt,
) -> Result<SubscriptionId, anyhow::Error> {
    let params = json!({ "transaction_hash": tx_hash });
    subscribe(ws, "starknet_subscribeTransactionStatus", params).await
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
    subscription_id: SubscriptionId,
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

    let subscription_id = subscribe_tx_status(&mut ws, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_successful_mint_notification(notification, tx_hash, subscription_id);
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
    assert_no_notifications(&mut ws).await;
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

    let dummy_tx_hash = Felt::ONE;
    subscribe_tx_status(&mut ws, &dummy_tx_hash).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();
    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);
    assert_ne!(tx_hash, dummy_tx_hash);

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
    subscribe_tx_status(&mut ws, &Felt::ONE).await.unwrap();

    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();

    // should work if subscribing before sending the tx
    let (mut ws_before_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id_before =
        subscribe_tx_status(&mut ws_before_tx, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    assert_no_notifications(&mut ws_before_tx).await;

    // should work even if subscribing after the tx was sent
    let (mut ws_after_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id_after =
        subscribe_tx_status(&mut ws_after_tx, &expected_tx_hash).await.unwrap();
    assert_no_notifications(&mut ws_after_tx).await;

    for (subscription_id, ws) in
        [(subscription_id_before, &mut ws_before_tx), (subscription_id_after, &mut ws_after_tx)]
    {
        let notification = receive_rpc_via_ws(ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
        assert_no_notifications(ws).await;
    }

    // move tx from pending to latest - expect no notifications
    devnet.create_block().await.unwrap();

    for ws in [&mut ws_before_tx, &mut ws_after_tx] {
        assert_no_notifications(ws).await;
    }
}

#[tokio::test]
async fn should_notify_in_on_transaction_mode() {
    let devnet_args = ["--block-generation-on", "transaction"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (address, mint_amount, expected_tx_hash) = first_mint_data();

    // should work if subscribing before sending the tx
    let (mut ws_before_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id_before =
        subscribe_tx_status(&mut ws_before_tx, &expected_tx_hash).await.unwrap();

    let tx_hash = devnet.mint(address, mint_amount).await;
    assert_eq!(tx_hash, expected_tx_hash);

    assert_no_notifications(&mut ws_before_tx).await;

    // should work even if subscribing after the tx was sent
    let (mut ws_after_tx, _) = connect_async(devnet.ws_url()).await.unwrap();
    let subscription_id_after =
        subscribe_tx_status(&mut ws_after_tx, &expected_tx_hash).await.unwrap();
    assert_no_notifications(&mut ws_after_tx).await;

    for (subscription_id, ws) in
        [(subscription_id_before, &mut ws_before_tx), (subscription_id_after, &mut ws_after_tx)]
    {
        let notification = receive_rpc_via_ws(ws).await.unwrap();
        assert_successful_mint_notification(notification, tx_hash, subscription_id);
        assert_no_notifications(ws).await;
    }

    // move tx from pending to latest - expect no notifications
    devnet.create_block().await.unwrap();

    for ws in [&mut ws_before_tx, &mut ws_after_tx] {
        assert_no_notifications(ws).await;
    }
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
    assert_successful_mint_notification(notification, tx_hash, subscription_id);

    devnet.mint(address, mint_amount).await;
    assert_no_notifications(&mut ws).await;
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
    assert_successful_mint_notification(notification, tx_hash, subscription_id);

    devnet.mint(address, mint_amount).await;
    assert_no_notifications(&mut ws).await;
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
    assert_successful_mint_notification(notification, tx_hash, subscription_id);

    devnet.abort_blocks(&BlockId::Number(1)).await.unwrap();

    // only expect reorg subscription
    let notification = receive_rpc_via_ws(&mut ws).await.unwrap();
    assert_eq!(notification["method"], "starknet_subscriptionReorg");
    assert_no_notifications(&mut ws).await;
}
