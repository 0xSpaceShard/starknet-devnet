pub mod common;

mod abort_blocks_tests {
    use serde_json::json;
    use server::api::json_rpc::error::ApiError;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{assert_tx_reverted, to_hex_felt};

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn abort_blocks(devnet: &BackgroundDevnet, starting_block_id: &BlockId) -> Vec<Felt> {
        let mut aborted_blocks = devnet
            .send_custom_rpc(
                "devnet_abortBlocks",
                json!({
                    "starting_block_id" : starting_block_id
                }),
            )
            .await
            .unwrap();

        let aborted_blocks = aborted_blocks["aborted"].take().as_array().unwrap().clone();

        aborted_blocks
            .into_iter()
            .map(|block_hash| serde_json::from_value(block_hash).unwrap())
            .collect()
    }

    async fn abort_blocks_error(devnet: &BackgroundDevnet, starting_block_id: &BlockId) {
        let aborted_blocks_error = devnet
            .send_custom_rpc(
                "devnet_abortBlocks",
                json!({
                "starting_block_id" : starting_block_id
                }),
            )
            .await
            .unwrap_err();

        assert!(aborted_blocks_error.message.contains("Block abortion failed"));
    }

    async fn assert_block_rejected(devnet: &BackgroundDevnet, block_hash: &Felt) {
        let block_after_abort = devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": to_hex_felt(block_hash)},
                }),
            )
            .await
            .unwrap();
        assert_eq!(block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_latest_block_with_hash() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let genesis_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;

        let new_block_hash = devnet.create_block().await.unwrap();
        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(new_block_hash)).await;
        assert_eq!(aborted_blocks, vec![new_block_hash]);

        // Check if the genesis block still has ACCEPTED_ON_L2 status
        let genesis_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": to_hex_felt(&genesis_block_hash)},
                }),
            )
            .await
            .unwrap();
        assert_eq!(genesis_block_after_abort["status"], "ACCEPTED_ON_L2".to_string());

        assert_block_rejected(&devnet, &new_block_hash).await;

        abort_blocks_error(&devnet, &BlockId::Hash(genesis_block_hash)).await;
    }

    #[tokio::test]
    async fn abort_two_blocks() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let first_block_hash = devnet.create_block().await.unwrap();
        let second_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(first_block_hash)).await;
        assert_eq!(json!(aborted_blocks), json!([second_block_hash, first_block_hash]));

        assert_block_rejected(&devnet, &first_block_hash).await;
        assert_block_rejected(&devnet, &second_block_hash).await;
    }

    #[tokio::test]
    async fn abort_block_with_transaction() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let mint_hash = devnet.mint(Felt::ONE, 100).await;

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(latest_block.block_hash)).await;
        assert_eq!(aborted_blocks, vec![latest_block.block_hash]);

        assert_tx_reverted(&mint_hash, &devnet.json_rpc_client, &["Block aborted manually"]).await;
    }

    #[tokio::test]
    async fn query_aborted_block_by_number_should_fail() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let new_block_hash = devnet.create_block().await.unwrap();
        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(new_block_hash)).await;
        assert_eq!(aborted_blocks, vec![new_block_hash]);
        assert_block_rejected(&devnet, &new_block_hash).await;

        let rpc_error = devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_number": 1},
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(rpc_error.message, ApiError::BlockNotFound.to_string())
    }

    #[tokio::test]
    async fn abort_block_state_revert() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let first_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let second_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(second_block.block_hash)).await;
        assert_eq!(aborted_blocks, vec![second_block.block_hash]);

        let balance = devnet
            .get_balance_latest(
                &Felt::from_hex(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(first_block.block_hash)).await;
        assert_eq!(aborted_blocks, vec![first_block.block_hash]);

        let balance = devnet
            .get_balance_latest(
                &Felt::from_hex(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), "0");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        let balance = devnet
            .get_balance_latest(
                &Felt::from_hex(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance.to_string(), DUMMY_AMOUNT.to_string());

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 2);
    }

    #[tokio::test]
    async fn abort_blocks_without_state_archive_capacity() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let new_block_hash = devnet.create_block().await.unwrap();
        abort_blocks_error(&devnet, &BlockId::Hash(new_block_hash)).await;
    }

    #[tokio::test]
    async fn abort_same_block_twice() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let first_block_hash = devnet.create_block().await.unwrap();
        let second_block_hash = devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&devnet, &BlockId::Hash(first_block_hash)).await;
        assert_eq!(aborted_blocks, vec![second_block_hash, first_block_hash]);

        abort_blocks_error(&devnet, &BlockId::Hash(first_block_hash)).await;
        abort_blocks_error(&devnet, &BlockId::Hash(second_block_hash)).await;
    }

    #[tokio::test]
    async fn abort_block_after_fork() {
        let origin_devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let fork_devnet = origin_devnet.fork_with_full_state_archive().await.unwrap();

        let fork_block_hash = fork_devnet.create_block().await.unwrap();

        let aborted_blocks = abort_blocks(&fork_devnet, &BlockId::Hash(fork_block_hash)).await;
        assert_eq!(aborted_blocks, vec![fork_block_hash]);

        abort_blocks_error(&fork_devnet, &BlockId::Hash(fork_block_hash)).await;
    }

    #[tokio::test]
    async fn abort_latest_blocks() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        for _ in 0..3 {
            devnet.create_block().await.unwrap();
        }
        for _ in 0..3 {
            abort_blocks(&devnet, &BlockId::Tag(BlockTag::Latest)).await;
        }
        abort_blocks_error(&devnet, &BlockId::Tag(BlockTag::Latest)).await; // Rolled back to genesis block, should not be possible to abort
    }
    #[tokio::test]
    async fn abort_pending_block() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--state-archive-capacity",
            "full",
            "--block-generation-on",
            "demand",
        ])
        .await
        .expect("Could not start Devnet");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        devnet.create_block().await.unwrap();
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let pending_balance = devnet
            .get_balance_by_tag(
                &Felt::from_hex(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                BlockTag::Pending,
            )
            .await
            .unwrap();
        assert_eq!(pending_balance, (2 * DUMMY_AMOUNT).into());

        abort_blocks(&devnet, &BlockId::Tag(BlockTag::Pending)).await;
        let latest_balance = devnet
            .get_balance_latest(
                &Felt::from_hex(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(latest_balance, DUMMY_AMOUNT.into());
    }
}
