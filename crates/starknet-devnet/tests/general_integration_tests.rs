// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_integration_tests {
    use hyper::{Body, StatusCode};
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{get_json_body, UniqueAutoDeletableFile};

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
        match devnet
            .post_json("/load".into(), Body::from(json!({ "path": too_big_path }).to_string()))
            .await
        {
            Ok(resp) => {
                assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
            }
            other => panic!("Unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn request_size_below_limit() {
        let limit = 100;
        let args = ["--request-body-size-limit", &limit.to_string()];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        // subtract enough so that the rest of the json body doesn't overflow the limit
        let ok_path = "0".repeat(limit - 20);
        match devnet
            .post_json("/load".into(), Body::from(json!({ "path": ok_path }).to_string()))
            .await
        {
            Ok(resp) => {
                assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
                let load_resp = get_json_body(resp).await;
                assert_eq!(load_resp, json!({ "error": "The file does not exist" }));
            }
            other => panic!("Unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_config() {
        // random values
        let dump_file = UniqueAutoDeletableFile::new("dummy");
        let mut expected_config = json!({
            "seed": 1,
            "total_accounts": 2,
            "account_contract_class_hash": "0x61dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f",
            "predeployed_accounts_initial_balance": "3",
            "start_time": 4,
            "gas_price": 5,
            "data_gas_price": 6,
            "chain_id": "SN_MAIN",
            "dump_on": "exit",
            "dump_path": dump_file.path,
            "state_archive": "full",
            "fork_config": null,
            "server_config": {
                "host": "0.0.0.0",
                // expected port added after spawning; determined by port-acquiring logic
                "timeout": 121,
                "request_body_size_limit": 1000
            }
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
            &serde_json::to_string(&expected_config["gas_price"]).unwrap(),
            "--data-gas-price",
            &serde_json::to_string(&expected_config["data_gas_price"]).unwrap(),
            "--chain-id",
            "MAINNET",
            "--dump-on",
            &expected_config["dump_on"].as_str().unwrap(),
            "--dump-path",
            &expected_config["dump_path"].as_str().unwrap(),
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
