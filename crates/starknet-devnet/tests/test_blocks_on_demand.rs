pub mod common;

mod blocks_on_demand_tests {

    use starknet_rs_core::types::FieldElement;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    
    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const TX_COUNT: u128 = 5;

    #[tokio::test]
    async fn blocks_on_demand() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");

        for _ in 0..TX_COUNT {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        // TODO: fix state 
        // let balance_before_block = devnet
        //     .get_balance(
        //         &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
        //         FeeUnit::WEI,
        //     )
        //     .await
        //     .unwrap();
        // assert_eq!(balance_before_block, FieldElement::from(0 as u128));

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 0);
        assert_eq!(latest_block.transactions.len(), 0);

        devnet.create_block().await.unwrap();
        let balance_after_block = devnet
            .get_balance(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
            )
            .await
            .unwrap();
        assert_eq!(balance_after_block, FieldElement::from((TX_COUNT * DUMMY_AMOUNT) as u128));

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, 1);
        assert_eq!(latest_block.transactions.len() as u128, TX_COUNT);
    }
}
