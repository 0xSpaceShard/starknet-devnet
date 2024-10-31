#![cfg(test)]
pub mod common;

mod websocket_subscription_support {
    use futures::StreamExt;
    use serde_json::json;
    use tokio_tungstenite::connect_async;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::send_text_rpc_via_ws;

    #[tokio::test]
    async fn subscribe_to_new_block_heads_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

        let subscription_confirmation =
            send_text_rpc_via_ws(&mut ws, "starknet_subscribeNewHeads", json!({})).await.unwrap();
        let subscription_id = subscription_confirmation["result"].as_i64().unwrap();

        let created_block_hash = devnet.create_block().await.unwrap();

        match ws.next().await {
            Some(Ok(msg)) => {
                let notification: serde_json::Value =
                    serde_json::from_str(&msg.into_text().unwrap()).unwrap();
                assert_eq!(notification["method"], "starknet_subscriptionNewHeads");
                assert_eq!(
                    notification["params"]["result"]["block_hash"].as_str().unwrap(),
                    created_block_hash.to_hex_string().as_str()
                );
                assert_eq!(notification["params"]["result"]["block_number"].as_i64().unwrap(), 1);
                assert_eq!(
                    notification["params"]["subscription_id"].as_i64().unwrap(),
                    subscription_id
                )
            }
            other => panic!("Invalid websocket response: {other:?}"),
        }
    }
}
