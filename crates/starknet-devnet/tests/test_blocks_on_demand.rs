pub mod common;

mod blocks_on_demand_tests {

    use starknet_rs_core::types::{BlockStatus, BlockTag, FieldElement};
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common;
    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::assert_tx_successful;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

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

    async fn assert_balance(devnet: &BackgroundDevnet, expected: FieldElement, tag: BlockTag) {
        let balance = devnet
            .get_balance_by_tag(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                tag,
            )
            .await
            .unwrap();
        assert_eq!(balance, expected);
    }

    #[tokio::test]
    async fn blocks_on_demand_states_and_blocks() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        let tx_count = 5;
        for _ in 0..tx_count {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(0_u128), BlockTag::Latest).await;

        devnet.create_block().await.unwrap();

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Latest)
            .await;

        assert_latest_block_with_transactions(&devnet, 1, tx_count).await;

        // check if pending_block was restarted
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(pending_block.block_number, 2);
        assert_eq!(pending_block.transactions.len(), 0);

        assert_pending_block_with_transactions(&devnet, 2, 0).await;
    }
}
