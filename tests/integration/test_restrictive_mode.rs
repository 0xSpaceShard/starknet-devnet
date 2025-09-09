use serde_json::json;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::errors::RpcError;

#[tokio::test]
async fn restrictive_mode_with_default_methods_doesnt_affect_other_functionality() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode"])
        .await
        .expect("Could not start Devnet");

    devnet.send_custom_rpc("devnet_getConfig", json!({})).await.unwrap();
}

#[tokio::test]
async fn restrictive_mode_with_default_methods() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode"])
        .await
        .expect("Could not start Devnet");

    let json_rpc_error = devnet
        .send_custom_rpc("devnet_mint", json!({ "address": "0x1", "amount": 1 }))
        .await
        .unwrap_err();

    assert_eq!(
        json_rpc_error,
        RpcError { code: -32604, message: "Method forbidden".into(), data: None }
    );
}

#[tokio::test]
async fn restrictive_mode_with_custom_methods() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--restrictive-mode",
        "devnet_load",
        "devnet_mint",
    ])
    .await
    .expect("Could not start Devnet");

    let json_rpc_error = devnet
        .send_custom_rpc("devnet_mint", json!({ "address": "0x1", "amount": 1 }))
        .await
        .unwrap_err();

    assert_eq!(
        json_rpc_error,
        RpcError { code: -32604, message: "Method forbidden".into(), data: None }
    );
}
