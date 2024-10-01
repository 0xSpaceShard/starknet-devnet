#![cfg(test)]
pub mod common;

mod websocket_support {
    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    use starknet_rs_core::types::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use tokio::net::TcpStream;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    use crate::common::background_devnet::BackgroundDevnet;

    async fn send_rpc_via_ws(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let text_body = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
            "params": params
        })
        .to_string();
        ws.send(tokio_tungstenite::tungstenite::Message::Text(text_body)).await?;

        let resp_raw =
            ws.next().await.ok_or(anyhow::Error::msg("No response in websocket stream"))??;
        let resp_body: serde_json::Value = serde_json::from_slice(&resp_raw.into_data())?;

        Ok(resp_body)
    }

    #[tokio::test]
    async fn mint_and_check_tx_via_websocket() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let mint_resp = send_rpc_via_ws(
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

        let tx = send_rpc_via_ws(
            &mut ws,
            "starknet_getTransactionByHash",
            json!({ "transaction_hash": tx_hash }),
        )
        .await
        .unwrap();

        assert!(tx["result"].is_object());
    }

    #[tokio::test]
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
            send_rpc_via_ws(
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
    async fn invalid_request() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let resp = send_rpc_via_ws(&mut ws, "devnet_mint", json!({})).await.unwrap();
        assert_eq!(resp["error"]["message"], "missing field `address`");
    }
}
