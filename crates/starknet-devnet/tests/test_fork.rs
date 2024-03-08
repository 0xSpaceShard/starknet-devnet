pub mod common;

mod fork_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;
    const SEPOLIA_URL: &str = "https://alpha-sepolia.starknet.io";
    const SEPOLIA_ACCOUNT_ADDRESS: &str = "0x0";
    const SEPOLIA_EXPECTED_BALANCE: &str = "0x0";
    const SEPOLIA_GENESIS_BLOCK: &str =
        "0x5c627d4aeb51280058bed93c7889bce78114d63baad1be0f0aeb32496d5f19c";

    #[tokio::test]
    #[ignore = "Not supported"]
    async fn test_forking_sepolia_genesis_block() {
        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", SEPOLIA_URL])
                .await
                .expect("Could not start Devnet");

        let fork_genesis_block = &fork_devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({ "block_id": SEPOLIA_GENESIS_BLOCK }),
            )
            .await["result"];

        assert_eq!(fork_genesis_block["block_number"], 0);
    }

    #[tokio::test]
    #[ignore = "Not supported"]
    async fn test_forking_sepolia_contract_call_get_balance() {
        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", SEPOLIA_URL])
                .await
                .expect("Could not start Devnet");

        let contract_address = FieldElement::from_hex_be(SEPOLIA_ACCOUNT_ADDRESS).unwrap();
        let retrieved_result = fork_devnet.get_balance(&contract_address).await.unwrap();

        let expected_balance = FieldElement::from_hex_be(SEPOLIA_EXPECTED_BALANCE).unwrap();
        assert_eq!(retrieved_result, expected_balance);
    }

    #[tokio::test]
    #[ignore = "Not supported"]
    async fn test_forking_local_genesis_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let latest_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", devnet.url.as_str()])
                .await
                .expect("Could not start Devnet");

        let fork_genesis_block = &fork_devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({ "block_id": latest_block["block_hash"] }),
            )
            .await["result"];
        assert_eq!(fork_genesis_block["block_number"], 0);

        let retrieved_result =
            fork_devnet.get_balance(&FieldElement::from(DUMMY_ADDRESS)).await.unwrap();
        let expected_balance = FieldElement::from(DUMMY_AMOUNT);

        assert_eq!(retrieved_result, expected_balance);
    }

    #[tokio::test]
    async fn test_forking_local_declare_deploy_fork_invoke() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        );

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let predeployed_account = Arc::new(predeployed_account);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // generate the address of the newly deployed contract
        let new_contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &[],
        );

        // fork devnet
        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", devnet.url.as_str()])
                .await
                .expect("Could not start Devnet");

        let (fork_signer, fork_account_address) = fork_devnet.get_first_predeployed_account().await;
        let fork_predeployed_account = SingleOwnerAccount::new(
            fork_devnet.clone_provider(),
            fork_signer,
            fork_account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        );

        // invoke on forked devnet
        let events_contract_call = vec![Call {
            to: new_contract_address,
            selector: get_selector_from_name("emit_event").unwrap(),
            calldata: vec![FieldElement::from(1u8)],
        }];

        let invoke_result = fork_predeployed_account
            .execute(events_contract_call.clone())
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        // check invoke transaction
        let invoke_tx = fork_devnet
            .json_rpc_client
            .get_transaction_by_hash(invoke_result.transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = invoke_tx
        {
            assert_eq!(invoke_v1.transaction_hash, invoke_result.transaction_hash);
        } else {
            panic!("Could not unpack the transaction from {invoke_tx:?}");
        }
    }
}
