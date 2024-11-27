#![cfg(test)]
pub mod common;

mod websocket_support {
    use serde_json::json;
    use starknet_rs_core::types::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use tokio_tungstenite::connect_async;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        assert_no_notifications, send_binary_rpc_via_ws, send_text_rpc_via_ws, subscribe,
    };

    #[tokio::test]
    /// Testing for all non-ws methods would be longsome, so we just test for one devnet_ and one
    /// starknet_ method
    async fn test_general_rpc_support_via_websocket_is_disabled() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let expected_resp =
            json!({"jsonrpc":"2.0", "id":0, "error":{"code":-32601, "message":"Method not found"}});

        assert_eq!(
            send_text_rpc_via_ws(&mut ws, "devnet_mint", json!({})).await.unwrap(),
            expected_resp,
        );

        assert_eq!(
            send_text_rpc_via_ws(&mut ws, "starknet_syncing", json!({})).await.unwrap(),
            expected_resp,
        );
    }

    #[tokio::test]
    #[ignore = "General RPC support via websocket is disabled"]
    async fn mint_and_check_tx_via_websocket() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let mint_resp = send_text_rpc_via_ws(
            &mut ws,
            "devnet_mint",
            json!({
                "address": "0x1",
                "amount": 100,
                "unit": "WEI",
            }),
        )
        .await
        .unwrap();

        let tx_hash = mint_resp["result"]["tx_hash"].as_str().unwrap();

        let tx = send_text_rpc_via_ws(
            &mut ws,
            "starknet_getTransactionByHash",
            json!({ "transaction_hash": tx_hash }),
        )
        .await
        .unwrap();

        assert!(tx["result"].is_object());
    }

    #[tokio::test]
    #[ignore = "General RPC support via websocket is disabled"]
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
            send_binary_rpc_via_ws(&mut ws, "starknet_getBlockWithTxs", block_specifier)
                .await
                .unwrap();
        assert_eq!(block_resp_after["result"]["block_number"], 1);
    }

    #[tokio::test]
    #[ignore = "General RPC support via websocket is disabled"]
    async fn multiple_ws_connections() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let iterations = 10;

        let mut ws_streams = vec![];
        for _ in 0..iterations {
            let (ws, _) = connect_async(devnet.ws_url()).await.unwrap();
            ws_streams.push(ws);
        }

        let dummy_address: &str = "0x1";
        let single_mint_amount = 10;
        for ws in &mut ws_streams {
            send_text_rpc_via_ws(
                ws,
                "devnet_mint",
                json!({ "address": dummy_address, "amount": single_mint_amount }),
            )
            .await
            .unwrap();
        }

        let balance = devnet
            .get_balance_latest(&Felt::from_hex_unchecked(dummy_address), FeeUnit::WEI)
            .await
            .unwrap();
        assert_eq!(balance, Felt::from(single_mint_amount * iterations));
    }

    #[tokio::test]
    #[ignore = "General RPC support via websocket is disabled"]
    async fn invalid_request() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let resp = send_text_rpc_via_ws(&mut ws, "devnet_mint", json!({})).await.unwrap();
        assert_eq!(resp["error"]["message"], "missing field `address`");
    }

    #[tokio::test]
    async fn restarting_should_forget_all_sockets() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        devnet.create_block().await.unwrap();

        subscribe(&mut ws, "starknet_subscribeNewHeads", json!({})).await.unwrap();

        devnet.restart().await;

        assert_no_notifications(&mut ws).await;
    }
}
