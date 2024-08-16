pub mod common;

mod test_restrictive_mode {
    use serde_json::json;
    use server::rpc_core::error::ErrorCode;
    use starknet_rs_core::types::Felt;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::reqwest_client::{
        GetReqwestSender, HttpEmptyResponseBody, PostReqwestSender,
    };

    #[tokio::test]
    async fn restrictive_mode_with_default_methods_doesnt_affect_other_functionality() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode"])
            .await
            .expect("Could not start Devnet");

        devnet
            .reqwest_client()
            .get_json_async("/config", None)
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap();
    }

    #[tokio::test]
    async fn restrictive_mode_with_default_methods() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode"])
            .await
            .expect("Could not start Devnet");
        let http_err = devnet
            .reqwest_client()
            .post_json_async(
                "/mint",
                json!({
                    "address": format!("0x{:x}", Felt::ONE),
                    "amount": 1
                }),
            )
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap_err();
        assert_eq!(http_err.status(), reqwest::StatusCode::FORBIDDEN);

        let json_rpc_error = devnet
            .send_custom_rpc(
                "devnet_mint",
                json!({
                    "address": format!("0x{:x}", Felt::ONE),
                    "amount": 1
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(json_rpc_error.code, ErrorCode::MethodForbidden);
    }

    #[tokio::test]
    async fn restrictive_mode_with_custom_methods() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--restrictive-mode",
            "/load",
            "devnet_mint",
        ])
        .await
        .expect("Could not start Devnet");
        let err = devnet
            .reqwest_client()
            .post_json_async("/load", json!({ "path": "dummy_path" }))
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap_err();

        assert_eq!(err.status(), reqwest::StatusCode::FORBIDDEN);

        let json_rpc_error = devnet
            .send_custom_rpc(
                "devnet_mint",
                json!({
                    "address": format!("0x{:x}", Felt::ONE),
                    "amount": 1
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(json_rpc_error.code, ErrorCode::MethodForbidden);
    }
}
