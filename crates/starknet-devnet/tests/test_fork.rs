pub mod common;

mod fork_tests {
    use serde_json::json;
    use starknet_rs_core::types::FieldElement;

    use crate::common::background_devnet::BackgroundDevnet;
    
    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const SEPOLIA_URL: &str = "https://alpha-sepolia.starknet.io";
    const SEPOLIA_ACCOUNT_ADDRESS: &str = "0x0";
    const SEPOLIA_EXPECTED_BALANCE: &str = "0x0";
    const SEPOLIA_GENESIS_BLOCK: &str = "0x5c627d4aeb51280058bed93c7889bce78114d63baad1be0f0aeb32496d5f19c";

    #[tokio::test]
    async fn test_forking_sepolia_genesis_block() {
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--fork-network",
            SEPOLIA_URL,
            "--accounts",
            "0",
        ])
        .await
        .expect("Could not start Devnet");
        
        let fork_genesis_block = &fork_devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": SEPOLIA_GENESIS_BLOCK }))
            .await["result"];

        assert_eq!(fork_genesis_block["block_number"], 0);
    }

    #[tokio::test]
    async fn test_forking_sepolia_contract_call_get_balance() {
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--fork-network",
            SEPOLIA_URL,
            "--accounts",
            "0",
        ])
        .await
        .expect("Could not start Devnet");

        // TODO: This should fail or get balance = 0? Maybe call for something different?
        let contract_address = FieldElement::from_hex_be(SEPOLIA_ACCOUNT_ADDRESS).unwrap();
        let retrieved_result = fork_devnet.get_balance(&contract_address).await.unwrap();

        let expected_balance = FieldElement::from_hex_be(SEPOLIA_EXPECTED_BALANCE).unwrap();
        assert_eq!(retrieved_result, expected_balance);
    }

    #[tokio::test]
    async fn test_forking_local_genesis_block() {
        let devnet: BackgroundDevnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        // devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let latest_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--fork-network",
            devnet.url.as_str(),
            // "--port", - is this needed?
            // DEVNET_DEFAULT_PORT,
            "--accounts",
            "0",
        ])
        .await
        .expect("Could not start Devnet");
    
        let fork_genesis_block = &fork_devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": latest_block["block_hash"] }))
            .await["result"];
        assert_eq!(fork_genesis_block["block_number"], 0);

        let retrieved_result = fork_devnet.get_balance(&FieldElement::from(DUMMY_ADDRESS)).await.unwrap();
        let expected_balance = FieldElement::from(DUMMY_AMOUNT);
        assert_eq!(retrieved_result, expected_balance);
    }
}
