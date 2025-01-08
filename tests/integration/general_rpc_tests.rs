use serde_json::json;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::RPC_PATH;
use crate::common::errors::RpcError;
use crate::common::reqwest_client::PostReqwestSender;

#[tokio::test]
async fn rpc_at_root() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let resp_root: serde_json::Value =
        devnet.reqwest_client().post_json_async("/", ()).await.unwrap();

    let resp_rpc: serde_json::Value =
        devnet.reqwest_client().post_json_async(RPC_PATH, ()).await.unwrap();

    assert_eq!(resp_root, resp_rpc);
}

#[tokio::test]
async fn rpc_returns_correct_spec_version() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let resp_body = devnet.send_custom_rpc("starknet_specVersion", json!([])).await.unwrap();
    match resp_body.as_str() {
        Some(received_ver) => assert_eq!(received_ver, "0.7.1"),
        _ => panic!("Invalid resp: {resp_body}"),
    }
}

#[tokio::test]
async fn rpc_returns_method_not_found() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    for invalid_method in ["invalid_method", "starknet_specVersion_butWrong"] {
        let rpc_error = devnet.send_custom_rpc(invalid_method, json!({})).await.unwrap_err();
        assert_eq!(
            rpc_error,
            RpcError { code: -32601, message: "Method not found".into(), data: None }
        );
    }
}

#[tokio::test]
async fn rpc_returns_invalid_params() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let rpc_error = devnet
        .send_custom_rpc("starknet_specVersion", json!({ "invalid_param": "unneeded_value" }))
        .await
        .unwrap_err();

    assert_eq!(rpc_error.code, -32602);
}
