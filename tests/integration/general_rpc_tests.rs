use serde_json::json;
use starknet_rs_core::types::{ConfirmedBlockId, SyncStatusType};
use starknet_rs_providers::Provider;
use starknet_rs_providers::jsonrpc::JsonRpcError;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::RPC_PATH;
use crate::common::errors::RpcError;
use crate::common::reqwest_client::PostReqwestSender;
use crate::common::utils::{assert_json_rpc_errors_equal, extract_json_rpc_error};

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
        Some(received_ver) => assert_eq!(received_ver, "0.9.0-rc.2"),
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

#[tokio::test]
async fn syncing_status_always_false() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    assert_eq!(devnet.json_rpc_client.syncing().await.unwrap(), SyncStatusType::NotSyncing);
}

#[tokio::test]
async fn storage_proof_request_should_always_return_error() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let devnet_storage_proof_msg = "Devnet doesn't support storage proofs";

    for (req_params, expected_code, expected_msg) in [
        (json!({}), -32602, "missing field `block_id`"),
        (json!({ "block_id": ConfirmedBlockId::Number(0) }), 42, devnet_storage_proof_msg),
        (json!({ "block_id": "latest" }), 42, devnet_storage_proof_msg),
        (json!({ "block_id": ConfirmedBlockId::Number(5) }), 24, "Block not found"),
    ] {
        let error =
            devnet.send_custom_rpc("starknet_getStorageProof", req_params).await.unwrap_err();
        assert_eq!(
            error,
            RpcError { code: expected_code.into(), message: expected_msg.into(), data: None }
        );
    }

    // Test with starknet-rs
    match devnet.json_rpc_client.get_storage_proof(ConfirmedBlockId::Latest, [], [], []).await {
        // Replace when this is accepted: https://github.com/xJonathanLEI/starknet-rs/pull/714
        // Err(ProviderError::StarknetError(StarknetError::StorageProofNotSupported)) => (),
        Err(e) => assert_json_rpc_errors_equal(
            extract_json_rpc_error(e).unwrap(),
            JsonRpcError { code: 42, message: devnet_storage_proof_msg.into(), data: None },
        ),
        other => panic!("Unexpected result: {other:?}"),
    }
}
