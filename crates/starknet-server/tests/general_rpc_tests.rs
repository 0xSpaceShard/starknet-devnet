// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_rpc_tests {
    use hyper::Body;
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::RPC_PATH;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn rpc_at_root() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let resp_root =
            devnet.post_json("/".into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_root_body = get_json_body(resp_root).await;

        let resp_rpc =
            devnet.post_json(RPC_PATH.into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_rpc_body = get_json_body(resp_rpc).await;

        assert_eq!(resp_root_body, resp_rpc_body);
    }

    const EXPECTED_VERSION: &str = "0.5.1";

    #[test]
    /// This test asserts that the spec files used in testing indeed match the expected version
    fn rpc_spec_using_correct_version() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path_to_spec_dir = format!("{manifest_dir}/test_data/spec/{EXPECTED_VERSION}");
        let spec_files = std::fs::read_dir(path_to_spec_dir).unwrap();

        // traverse all json files in the rpc spec dir and assert they all have the expected version
        for spec_file in
            spec_files.filter(|f| f.as_ref().unwrap().path().extension().unwrap() == "json")
        {
            let spec_file_path = spec_file.unwrap().path();
            let spec_file_path = spec_file_path.to_str().unwrap();
            let spec_reader = std::fs::File::open(spec_file_path).unwrap();
            let spec_content: serde_json::Value = serde_json::from_reader(spec_reader).unwrap();
            match spec_content
                .get("info")
                .and_then(|info| info.get("version"))
                .and_then(|ver| ver.as_str())
            {
                Some(EXPECTED_VERSION) => (),
                other => panic!("Invalid version in {spec_file_path}: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn rpc_returns_correct_spec_version() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let resp_body = devnet.send_custom_rpc("starknet_specVersion", json!([])).await;
        match resp_body.get("result").and_then(|val| val.as_str()) {
            Some(received_ver) => assert_eq!(received_ver, EXPECTED_VERSION),
            _ => panic!("Invalid resp: {resp_body}"),
        }
    }

    #[tokio::test]
    async fn rpc_returns_method_not_found() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        for invalid_method in ["invalid_method", "starknet_specVersion_butWrong"] {
            let resp_body = devnet.send_custom_rpc(invalid_method, json!({})).await;

            match resp_body
                .get("error")
                .and_then(|err| err.get("code"))
                .and_then(|val| val.as_i64())
            {
                Some(received) => {
                    assert_eq!(received, server::rpc_core::error::ErrorCode::MethodNotFound.code())
                }
                _ => panic!("Invalid resp: {resp_body}"),
            }
        }
    }

    #[tokio::test]
    async fn rpc_returns_invalid_params() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp_body = devnet
            .send_custom_rpc(
                "starknet_specVersion",
                json!({
                    "invalid_param": "unneeded_value",
                }),
            )
            .await;

        match resp_body.get("error").and_then(|err| err.get("code")).and_then(|val| val.as_i64()) {
            Some(received) => {
                assert_eq!(received, server::rpc_core::error::ErrorCode::InvalidParams.code())
            }
            _ => panic!("Invalid resp: {resp_body}"),
        }
    }
}
