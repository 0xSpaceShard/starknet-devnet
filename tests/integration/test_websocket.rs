use serde_json::json;
use starknet_rs_core::types::{Felt, Transaction};
use tokio_tungstenite::connect_async;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::{
    FeeUnit, assert_no_notifications, send_binary_rpc_via_ws, send_text_rpc_via_ws, subscribe,
};

#[tokio::test]
async fn mint_and_check_tx_via_websocket() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mint_resp = send_text_rpc_via_ws(
        &mut ws,
        "devnet_mint",
        json!({ "address": "0x1", "amount": 100, "unit": "FRI" }),
    )
    .await
    .unwrap();

    let tx_hash = Felt::from_hex_unchecked(mint_resp["result"]["tx_hash"].as_str().unwrap());

    let tx_raw = send_text_rpc_via_ws(
        &mut ws,
        "starknet_getTransactionByHash",
        json!({ "transaction_hash": tx_hash }),
    )
    .await
    .unwrap();

    let tx: Transaction = serde_json::from_value(tx_raw["result"].clone()).unwrap();
    assert_eq!(tx.transaction_hash(), &tx_hash);
}

#[tokio::test]
async fn create_block_via_binary_ws_message() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let block_specifier = json!({ "block_id": "latest" });
    let block_resp_before =
        send_binary_rpc_via_ws(&mut ws, "starknet_getBlockWithTxs", block_specifier.clone())
            .await
            .unwrap();
    assert_eq!(block_resp_before["result"]["block_number"], 0);

    let creation_resp =
        send_binary_rpc_via_ws(&mut ws, "devnet_createBlock", json!({})).await.unwrap();
    assert!(creation_resp["result"].is_object());

    let block_resp_after =
        send_binary_rpc_via_ws(&mut ws, "starknet_getBlockWithTxs", block_specifier).await.unwrap();
    assert_eq!(block_resp_after["result"]["block_number"], 1);
}

#[tokio::test]
async fn multiple_ws_connections() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let iterations = 2;

    let mut ws_streams = vec![];
    for _ in 0..iterations {
        let (ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        ws_streams.push(ws);
    }

    let dummy_address: &str = "0x1";
    let single_mint_amount = 10;
    for ws in &mut ws_streams {
        let res = send_text_rpc_via_ws(
            ws,
            "devnet_mint",
            json!({ "address": dummy_address, "amount": single_mint_amount }),
        )
        .await
        .unwrap();
        println!("DEBUG res: {res}");
    }

    let balance = devnet
        .get_balance_latest(&Felt::from_hex_unchecked(dummy_address), FeeUnit::Fri)
        .await
        .unwrap();
    assert_eq!(balance, Felt::from(single_mint_amount * iterations));
}

#[tokio::test]
async fn invalid_general_rpc_request() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let resp = send_text_rpc_via_ws(&mut ws, "devnet_mint", json!({})).await.unwrap();
    assert_eq!(resp["error"]["message"], "missing field `address`");
}

#[tokio::test]
async fn restarting_should_forget_all_websocket_subscriptions() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    devnet.create_block().await.unwrap();

    subscribe(&mut ws, "starknet_subscribeNewHeads", json!({})).await.unwrap();

    devnet.restart().await;

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_invalid_rpc_methods_via_ws() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    for method in [
        "starknet_invalid",
        "definitely_invalid",
        "devnet_invalid",
        "starknet_subscribeInvalid",
        "starknet_unsubscribeInvalid",
    ] {
        let resp = send_text_rpc_via_ws(&mut ws, method, json!({})).await.unwrap();
        assert_eq!(
            resp,
            json!({
                "jsonrpc": "2.0",
                "id": 0,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                }
            })
        );
    }
}
