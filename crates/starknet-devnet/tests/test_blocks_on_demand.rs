pub mod common;

mod blocks_on_demand_tests {

    use starknet_rs_core::types::FieldElement;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const TX_COUNT: u128 = 1; // TODO: fix nonce and change back to 5

    #[tokio::test]
    async fn blocks_on_demand() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");

        let balance_from_pending_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                true,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_pending_state, FieldElement::from(0_u128));
        let balance_from_latest_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                false,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_latest_state, FieldElement::from(0_u128));

        for _ in 0..TX_COUNT {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        let balance_from_pending_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                true,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_pending_state, FieldElement::from(TX_COUNT * DUMMY_AMOUNT));
        let balance_from_latest_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                false,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_latest_state, FieldElement::from(0_u128));

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 0);
        assert_eq!(latest_block.transactions.len(), 0);

        devnet.create_block().await.unwrap();

        let balance_from_pending_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                true,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_pending_state, FieldElement::from(TX_COUNT * DUMMY_AMOUNT));

        let balance_from_latest_state = devnet
            .get_balance_pending_block(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                false,
            )
            .await
            .unwrap();
        assert_eq!(balance_from_latest_state, FieldElement::from(TX_COUNT * DUMMY_AMOUNT));

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 1);
        assert_eq!(latest_block.transactions.len() as u128, TX_COUNT);
    }
}
