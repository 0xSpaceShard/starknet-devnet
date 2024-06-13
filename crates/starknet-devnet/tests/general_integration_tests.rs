// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_integration_tests {
    use reqwest::StatusCode;
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::reqwest_client::{HttpEmptyResponseBody, PostReqwestSender};
    use crate::common::utils::UniqueAutoDeletableFile;

    #[tokio::test]
    /// Asserts that a background instance can be spawned
    async fn spawnable() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn too_big_request_rejected() {
        let limit = 1_000;
        let args = ["--request-body-size-limit", &limit.to_string()];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        let too_big_path = "a".repeat(limit);
        let reqwest_error =
            PostReqwestSender::<serde_json::Value, HttpEmptyResponseBody>::post_json_async(
                devnet.reqwest_client(),
                "/load",
                json!({"path": too_big_path}),
            )
            .await
            .expect_err("Request should have been rejected");

        assert_eq!(reqwest_error.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn request_size_below_limit() {
        let limit = 100;
        let args = ["--request-body-size-limit", &limit.to_string()];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        // subtract enough so that the rest of the json body doesn't overflow the limit
        let ok_path = "0".repeat(limit - 20);
        let error = devnet
            .reqwest_client()
            .post_json_async("/load", json!({ "path": ok_path }))
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap_err();

        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            error.error_message(),
            json!({ "error": "The file does not exist" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_config() {
        // random values
        let dump_file = UniqueAutoDeletableFile::new("dummy");
        let mut expected_config = json!({
            "seed": 1,
            "total_accounts": 2,
            "account_contract_class_hash": "0xe2eb8f5672af4e6a4e8a8f1b44989685e668489b0a25437733756c5a34a1d6",
            "predeployed_accounts_initial_balance": "3",
            "start_time": 4,
            "gas_price_wei": 5,
            "gas_price_strk": 7,
            "data_gas_price_wei": 6,
            "data_gas_price_strk": 8,
            "chain_id": "SN_MAIN",
            "dump_on": "exit",
            "dump_path": dump_file.path,
            "state_archive": "full",
            "fork_config": {
                "url": null,
                "block_number": null,
            },
            "server_config": {
                "host": "0.0.0.0",
                // expected port added after spawning; determined by port-acquiring logic
                "timeout": 121,
                "request_body_size_limit": 1000,
            },
            "blocks_on_demand": true,
            "lite_mode": false,
            "disable_account_impersonation": false,
        });

        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--seed",
            &serde_json::to_string(&expected_config["seed"]).unwrap(),
            "--accounts",
            &serde_json::to_string(&expected_config["total_accounts"]).unwrap(),
            "--initial-balance",
            expected_config["predeployed_accounts_initial_balance"].as_str().unwrap(),
            "--start-time",
            &serde_json::to_string(&expected_config["start_time"]).unwrap(),
            "--gas-price",
            &serde_json::to_string(&expected_config["gas_price_wei"]).unwrap(),
            "--gas-price-strk",
            &serde_json::to_string(&expected_config["gas_price_strk"]).unwrap(),
            "--data-gas-price",
            &serde_json::to_string(&expected_config["data_gas_price_wei"]).unwrap(),
            "--data-gas-price-strk",
            &serde_json::to_string(&expected_config["data_gas_price_strk"]).unwrap(),
            "--chain-id",
            "MAINNET",
            "--dump-on",
            &expected_config["dump_on"].as_str().unwrap(),
            "--dump-path",
            &expected_config["dump_path"].as_str().unwrap(),
            "--blocks-on-demand",
            "--state-archive-capacity",
            &expected_config["state_archive"].as_str().unwrap(),
            "--host",
            expected_config["server_config"]["host"].as_str().unwrap(),
            "--timeout",
            &serde_json::to_string(&expected_config["server_config"]["timeout"]).unwrap(),
            "--request-body-size-limit",
            &serde_json::to_string(&expected_config["server_config"]["request_body_size_limit"])
                .unwrap(),
        ])
        .await
        .unwrap();

        expected_config["server_config"]["port"] = devnet.port.into();

        let fetched_config = devnet.get_config().await.unwrap();
        assert_eq!(fetched_config, expected_config);
    }
}
