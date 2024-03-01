pub mod common;

mod fork_tests {
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;

    const ORIGIN_URL: &str = "https://alpha4.starknet.io";
    const ALPHA_GOERLI_PREDEPLOYED_ACCOUNT_ADDRESS: &str = "0x0";
    const ALPHA_GOERLI_EXPECTED_BALANCE: &str = "0x0";
    const ALPHA_GOERLI_GENESIS_BLOCK: &str = "0x7d328a71faf48c5c3857e99f20a77b18522480956d1cd5bff1ff2df3c8b427b";

    #[tokio::test]
    async fn test_forking_genesis_block() {
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--fork-network",
            ORIGIN_URL,
            "--accounts",
            "0",
        ])
        .await
        .expect("Could not start Devnet");
        
        let genesis_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": ALPHA_GOERLI_GENESIS_BLOCK }))
            .await["result"];

        // TODO: add some asserts like block number = 0
        println!("genesis_block {:?}", genesis_block);
    }

    #[tokio::test]
    async fn test_forking_contract_call_get_balance() {
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--fork-network",
            ORIGIN_URL,
            "--accounts",
            "0",
        ])
        .await
        .expect("Could not start Devnet");

        // TODO: This should fail or get balance = 0? Maybe call for something different?
        let contract_address = FieldElement::from_hex_be(ALPHA_GOERLI_PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let retrieved_result = devnet.get_balance(&contract_address).await.unwrap();

        let expected_balance = FieldElement::from_hex_be(ALPHA_GOERLI_EXPECTED_BALANCE.as_str()).unwrap();
        assert_eq!(retrieved_result, expected_balance);
    }
}
