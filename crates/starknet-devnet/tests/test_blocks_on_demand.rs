pub mod common;

mod blocks_on_demand_tests {

    use starknet_rs_core::types::{BlockId, BlockStatus, BlockTag, FieldElement, MaybePendingStateUpdate};
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::assert_tx_successful;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const TX_COUNT: u128 = 5;

    async fn assert_latest_block_with_transactions(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions_count: u128,
    ) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(latest_block.transactions.len() as u128, transactions_count);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);

        for tx_hash in latest_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_transactions(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions_count: u128,
    ) {
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(pending_block.block_number, block_number);
        assert_eq!(pending_block.transactions.len() as u128, transactions_count);
        assert_eq!(pending_block.status, BlockStatus::Pending);
    }

    async fn assert_balance(
        devnet: &BackgroundDevnet,
        expected: FieldElement,
        pending_state: bool,
    ) {
        let balance = devnet
            .get_balance_pending_state(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                pending_state,
            )
            .await
            .unwrap();
        assert_eq!(balance, expected);
    }

    #[tokio::test]
    async fn blocks_on_demand_states_and_blocks() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");

        for _ in 0..TX_COUNT {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        assert_balance(&devnet, FieldElement::from(TX_COUNT * DUMMY_AMOUNT), true).await;
        assert_balance(&devnet, FieldElement::from(0_u128), false).await;

        devnet.create_block().await.unwrap();

        assert_balance(&devnet, FieldElement::from(TX_COUNT * DUMMY_AMOUNT), true).await;
        assert_balance(&devnet, FieldElement::from(TX_COUNT * DUMMY_AMOUNT), false).await;

        assert_latest_block_with_transactions(&devnet, 1, TX_COUNT).await;

        // check if pending_block was restarted
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(pending_block.block_number, 2);
        assert_eq!(pending_block.transactions.len(), 0);

        assert_pending_block_with_transactions(&devnet, 2, 0).await;
    }

    #[tokio::test]
    async fn get_state_update() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");

        devnet.create_block().await.unwrap();

        let state_update_pending_block =
            devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Pending)).await.unwrap();
        match state_update_pending_block {
            MaybePendingStateUpdate::PendingUpdate(_) => (),
            other => panic!("Unexpected result: {other:?}"),
        }

        let state_update_latest_block =
            devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Latest)).await.unwrap();
        match state_update_latest_block {
            MaybePendingStateUpdate::Update(_) => (),
            other => panic!("Unexpected result: {other:?}"),
        }
    }
}
