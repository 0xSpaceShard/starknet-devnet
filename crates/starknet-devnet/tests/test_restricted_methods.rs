pub mod common;

mod test_restricted_methods {
    use serde_json::json;
    use starknet_rs_core::types::FieldElement;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::reqwest_client::{HttpEmptyResponseBody, PostReqwestSender};

    #[tokio::test]
    async fn restrictive_mode_with_default_methods() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode"])
            .await
            .expect("Could not start Devnet");
        let err = devnet
            .reqwest_client()
            .post_json_async(
                "/mint",
                json!({
                    "address": format!("0x{:x}",FieldElement::ONE),
                    "amount": 1
                }),
            )
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap_err();

        assert_eq!(err.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn restrictive_mode_with_custom_methods() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--restrictive-mode", "/load"])
            .await
            .expect("Could not start Devnet");
        let err = devnet
            .reqwest_client()
            .post_json_async("/load", json!({ "path": "dummy_path" }))
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap_err();

        assert_eq!(err.status(), reqwest::StatusCode::FORBIDDEN);
    }
}
