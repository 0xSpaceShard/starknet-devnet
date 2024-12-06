#![cfg(test)]
// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_rpc_tests {
    use serde_json::json;
    use server::api::json_rpc::RPC_SPEC_VERSION;
    use server::rpc_core::error::RpcError;
    use starknet_rs_core::types::BlockId;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::RPC_PATH;
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
            Some(received_ver) => assert_eq!(received_ver, RPC_SPEC_VERSION),
            _ => panic!("Invalid resp: {resp_body}"),
        }
    }

    #[tokio::test]
    async fn rpc_returns_method_not_found() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        for invalid_method in ["invalid_method", "starknet_specVersion_butWrong"] {
            let rpc_error = devnet.send_custom_rpc(invalid_method, json!({})).await.unwrap_err();
            assert_eq!(rpc_error.code, server::rpc_core::error::ErrorCode::MethodNotFound);
        }
    }

    #[tokio::test]
    async fn rpc_returns_invalid_params() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let rpc_error = devnet
            .send_custom_rpc("starknet_specVersion", json!({ "invalid_param": "unneeded_value" }))
            .await
            .unwrap_err();

        assert_eq!(rpc_error.code, server::rpc_core::error::ErrorCode::InvalidParams);
    }

    #[tokio::test]
    async fn storage_proof_request_should_always_return_error() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        devnet.create_block().await.unwrap();

        for (req_params, expected_code, expected_msg) in [
            (json!({}), -32602, "missing field `block_id`"),
            (
                json!({ "block_id": BlockId::Number(0) }),
                42,
                "Devnet doesn't support storage proofs",
            ),
            (json!({ "block_id": "latest" }), 42, "Devnet doesn't support storage proofs"),
            (json!({ "block_id": BlockId::Number(5) }), 24, "Block not found"),
        ] {
            let error =
                devnet.send_custom_rpc("starknet_getStorageProof", req_params).await.unwrap_err();
            assert_eq!(
                error,
                RpcError { code: expected_code.into(), message: expected_msg.into(), data: None }
            );
        }
    }
}
