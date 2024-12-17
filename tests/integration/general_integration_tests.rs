pub mod general_integration_tests {
    use reqwest::StatusCode;
    use serde_json::json;
    use starknet_rs_core::types::{BlockId, BlockTag};
    use starknet_rs_core::utils::{get_storage_var_address, parse_cairo_short_string};
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
        ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::common::errors::RpcError;
    use crate::common::reqwest_client::{HttpEmptyResponseBody, PostReqwestSender};
    use crate::common::utils::{to_hex_felt, UniqueAutoDeletableFile};

    #[tokio::test]
    /// Asserts that a background instance can be spawned
    async fn spawnable() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn too_big_request_rejected_via_non_rpc() {
        let limit = 1_000;
        let args = ["--request-body-size-limit", &limit.to_string()];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        let too_long_path = "a".repeat(limit + 100);
        let err = PostReqwestSender::<serde_json::Value, HttpEmptyResponseBody>::post_json_async(
            devnet.reqwest_client(),
            "/load",
            json!({"path": too_long_path}),
        )
        .await
        .expect_err("Request should have been rejected");

        assert_eq!(err.status(), StatusCode::PAYLOAD_TOO_LARGE);

        // subtract enough so that the rest of the json body doesn't overflow the limit
        let nonexistent_path = "a".repeat(limit - 100);
        let err = PostReqwestSender::<serde_json::Value, HttpEmptyResponseBody>::post_json_async(
            devnet.reqwest_client(),
            "/load",
            json!({"path": nonexistent_path}),
        )
        .await
        .expect_err("Request should have been rejected");

        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_message(), json!({ "error": "The file does not exist" }).to_string());
    }

    #[tokio::test]
    async fn too_big_request_rejected_via_rpc() {
        let limit = 1_000;
        let args = ["--request-body-size-limit", &limit.to_string()];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        let too_long_path = "a".repeat(limit + 100);
        let error = devnet
            .send_custom_rpc("devnet_load", serde_json::json!({ "path": too_long_path }))
            .await
            .expect_err("Request should have been rejected");

        assert_eq!(
            error.code,
            ErrorCode::ServerError(StatusCode::PAYLOAD_TOO_LARGE.as_u16().into())
        );

        // subtract enough so that the rest of the json body doesn't overflow the limit
        let nonexistent_path = "a".repeat(limit - 100);
        let error = devnet
            .send_custom_rpc("devnet_load", serde_json::json!({ "path": nonexistent_path }))
            .await
            .expect_err("Request should have been rejected");

        assert_eq!(
            error,
            RpcError { code: -1, message: "The file does not exist".into(), data: None }
        );
    }

    #[tokio::test]
    async fn test_config() {
        // random values
        let dump_file = UniqueAutoDeletableFile::new("dummy");
        let mut expected_config = json!({
            "seed": 1,
            "total_accounts": 2,
            "account_contract_class_hash": CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
            "predeployed_accounts_initial_balance": "3",
            "start_time": 4,
            "gas_price_wei": 5,
            "gas_price_fri": 7,
            "data_gas_price_wei": 6,
            "data_gas_price_fri": 8,
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
                "restricted_methods": null,
            },
            "block_generation_on": "demand",
            "lite_mode": false,
            "eth_erc20_class_hash": to_hex_felt(&CAIRO_1_ERC20_CONTRACT_CLASS_HASH),
            "strk_erc20_class_hash": to_hex_felt(&CAIRO_1_ERC20_CONTRACT_CLASS_HASH),
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
            "--gas-price-fri",
            &serde_json::to_string(&expected_config["gas_price_fri"]).unwrap(),
            "--data-gas-price",
            &serde_json::to_string(&expected_config["data_gas_price_wei"]).unwrap(),
            "--data-gas-price-fri",
            &serde_json::to_string(&expected_config["data_gas_price_fri"]).unwrap(),
            "--chain-id",
            "MAINNET",
            "--dump-on",
            &expected_config["dump_on"].as_str().unwrap(),
            "--dump-path",
            &expected_config["dump_path"].as_str().unwrap(),
            "--block-generation-on",
            "demand",
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

        let fetched_config = devnet.get_config().await;
        assert_eq!(fetched_config, expected_config);
    }

    #[tokio::test]
    async fn predeployed_erc20_tokens_have_expected_storage() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        for (token_address, var_name, expected_value) in [
            (ETH_ERC20_CONTRACT_ADDRESS, "ERC20_name", "Ether"),
            (ETH_ERC20_CONTRACT_ADDRESS, "ERC20_symbol", "ETH"),
            (STRK_ERC20_CONTRACT_ADDRESS, "ERC20_name", "StarkNet Token"),
            (STRK_ERC20_CONTRACT_ADDRESS, "ERC20_symbol", "STRK"),
        ] {
            let actual_value = devnet
                .json_rpc_client
                .get_storage_at(
                    token_address,
                    get_storage_var_address(var_name, &[]).unwrap(),
                    BlockId::Tag(BlockTag::Latest),
                )
                .await
                .unwrap();

            assert_eq!(parse_cairo_short_string(&actual_value).unwrap().as_str(), expected_value);
        }
    }
}
