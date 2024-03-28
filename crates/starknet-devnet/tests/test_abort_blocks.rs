pub mod common;

mod abort_blocks_tests {
    use hyper::Body;
    use serde_json::json;
    use server::api::json_rpc::error::ApiError;
    use starknet_rs_core::types::FieldElement;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{assert_tx_reverted, get_json_body, to_hex_felt};

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn abort_blocks(
        devnet: &BackgroundDevnet,
        starting_block_hash: &FieldElement,
    ) -> Vec<FieldElement> {
        let abort_blocks_resp = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": to_hex_felt(starting_block_hash) }).to_string(),
                ),
            )
            .await
            .unwrap();

        let mut aborted_blocks = get_json_body(abort_blocks_resp).await;
        let aborted_blocks = aborted_blocks["aborted"].take().as_array().unwrap().clone();

        aborted_blocks
            .into_iter()
            .map(|block_hash| serde_json::from_value(block_hash).unwrap())
            .collect()
    }

    async fn abort_blocks_error(devnet: &BackgroundDevnet, starting_block_hash: &FieldElement) {
        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(
                    json!({ "starting_block_hash": to_hex_felt(starting_block_hash) }).to_string(),
                ),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert!(aborted_blocks["error"].to_string().starts_with("\"The block abortion failed"));
    }

    async fn assert_block_rejected(devnet: &BackgroundDevnet, block_hash: &FieldElement) {
        let block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": to_hex_felt(block_hash)},
                }),
            )
            .await["result"];
        assert_eq!(block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_latest_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let first_block_hash = devnet.create_block().await.unwrap();
        let second_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &second_block_hash).await;
        assert_eq!(aborted_blocks, vec![second_block_hash]);

        let first_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": to_hex_felt(&first_block_hash)},
                }),
            )
            .await["result"];
        assert_eq!(first_block_after_abort["status"], "ACCEPTED_ON_L2".to_string());

        assert_block_rejected(&devnet, &second_block_hash).await;
    }

    #[tokio::test]
    async fn abort_two_blocks() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let first_block_hash = devnet.create_block().await.unwrap();
        let second_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &first_block_hash).await;
        assert_eq!(json!(aborted_blocks), json!([second_block_hash, first_block_hash]));

        assert_block_rejected(&devnet, &first_block_hash).await;
        assert_block_rejected(&devnet, &second_block_hash).await;
    }

    #[tokio::test]
    async fn abort_block_with_transaction() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let mint_hash = devnet.mint(FieldElement::ONE, 100).await;

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &latest_block.block_hash).await;
        assert_eq!(aborted_blocks, vec![latest_block.block_hash]);

        assert_tx_reverted(&mint_hash, &devnet.json_rpc_client, &["Block aborted manually"]).await;
    }

    #[tokio::test]
    async fn query_aborted_block_by_number_should_fail() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.create_block().await.unwrap();
        let second_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &second_block_hash).await;
        assert_eq!(aborted_blocks, vec![second_block_hash]);
        assert_block_rejected(&devnet, &second_block_hash).await;

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
        let first_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let second_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &second_block.block_hash).await;
        assert_eq!(aborted_blocks, vec![second_block.block_hash]);

        let balance = devnet
            .get_balance(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

        let aborted_blocks = abort_blocks(&devnet, &first_block.block_hash).await;
        assert_eq!(aborted_blocks, vec![first_block.block_hash]);

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

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 1);
    }

    #[tokio::test]
    async fn abort_blocks_without_state_archive_capacity() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let first_block_hash = devnet.create_block().await.unwrap();
        abort_blocks_error(&devnet, &first_block_hash).await;
    }

    #[tokio::test]
    async fn abort_same_block_twice() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.create_block().await.unwrap();

        let second_block_hash = devnet.create_block().await.unwrap();
        let third_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &second_block_hash).await;
        assert_eq!(aborted_blocks, vec![third_block_hash, second_block_hash]);

        abort_blocks_error(&devnet, &second_block_hash).await;
        abort_blocks_error(&devnet, &third_block_hash).await;
    }
}
