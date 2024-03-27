pub mod common;

mod abort_blocks_tests {
    use hyper::Body;
    use serde_json::json;
    use server::api::json_rpc::error::ApiError;
    use starknet_rs_core::types::{FieldElement, TransactionStatus};
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn abort_latest_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": second_block["block_hash"] }).to_string(),
                ),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);

        let first_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": first_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(first_block_after_abort["status"], "ACCEPTED_ON_L2".to_string());

        let second_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": second_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(second_block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_two_blocks() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "starting_block_hash": first_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);
        assert_eq!(aborted_blocks["aborted"][1], first_block["block_hash"]);

        let first_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": first_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(first_block_after_abort["status"], "REJECTED".to_string());

        let second_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": second_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(second_block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_block_with_transaction() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let mint_hash = devnet.mint(FieldElement::ONE, 100).await;

        let latest_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": latest_block["block_hash"] }).to_string(),
                ),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], latest_block["block_hash"]);

        let tx_status_after_abort =
            devnet.json_rpc_client.get_transaction_status(mint_hash).await.unwrap();

        assert_eq!(
            tx_status_after_abort,
            TransactionStatus::AcceptedOnL2(
                starknet_rs_core::types::TransactionExecutionStatus::Reverted
            )
        );
    }

    #[tokio::test]
    async fn query_aborted_block_by_number_should_fail() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": second_block["block_hash"] }).to_string(),
                ),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);

        let second_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": second_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(second_block_after_abort["status"], "REJECTED".to_string());

        let second_block_after_abort_by_number = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_number": 1},
                }),
            )
            .await;
        assert_eq!(
            second_block_after_abort_by_number["error"]["message"],
            ApiError::BlockNotFound.to_string()
        )
    }

    #[tokio::test]
    async fn abort_block_state_revert() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": second_block["block_hash"] }).to_string(),
                ),
            )
            .await
            .unwrap();
        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);

        let balance = devnet
            .get_balance(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "starting_block_hash": first_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();
        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], first_block["block_hash"]);

        let balance = devnet
            .get_balance(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), "0");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        let balance = devnet
            .get_balance(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

        let latest_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_eq!(latest_block["block_number"], 1);
    }

    #[tokio::test]
    async fn abort_blocks_without_state_archive_capacity() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "starting_block_hash": first_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert!(aborted_blocks["error"].to_string().starts_with("\"The block abortion failed"));
    }
}
