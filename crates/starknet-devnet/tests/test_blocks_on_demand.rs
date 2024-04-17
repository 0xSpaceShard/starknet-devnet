pub mod common;

mod blocks_on_demand_tests {

    use starknet_rs_core::types::FieldElement;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const TX_COUNT: u128 = 1; // TODO: fix nonce and change back to 5

    async fn assert_latest_block(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions_count: u128,
    ) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(latest_block.transactions.len() as u128, transactions_count);
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
    async fn blocks_on_demand_mint() {
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

        assert_latest_block(&devnet, 1, TX_COUNT).await;
    }

    // TODO: add invoke/call test?
    // TODO: add dump/load test with block on demand mode
}
