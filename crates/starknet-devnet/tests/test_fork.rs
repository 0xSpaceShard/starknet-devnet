pub mod common;

mod fork_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, ContractClass, ExecutionResult, FieldElement, FunctionCall,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        assert_cairo1_classes_equal, get_simple_contract_in_sierra_and_compiled_class_hash,
        resolve_path,
    };

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
        let origin_devnet = BackgroundDevnet::spawn().await.unwrap(); // TODO state archive capacity?

        // change state and create block (without the block, there is nothing to fork from since
        // there is no genesis block by default)
        origin_devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let latest_block = &origin_devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let block_resp = &fork_devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({ "block_id": { "block_hash": latest_block["block_hash"] } }),
            )
            .await;
        assert_eq!(block_resp["result"]["block_number"], 0);

        let retrieved_result =
            fork_devnet.get_balance(&FieldElement::from(DUMMY_ADDRESS)).await.unwrap();
        let expected_balance = FieldElement::from(DUMMY_AMOUNT);

        assert_eq!(retrieved_result, expected_balance);
    }

    async fn get_contract_balance(
        devnet: &BackgroundDevnet,
        contract_address: FieldElement,
    ) -> FieldElement {
        let contract_call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_balance").unwrap(),
            calldata: vec![],
        };
        match devnet.json_rpc_client.call(contract_call, BlockId::Tag(BlockTag::Latest)).await {
            Ok(res) => {
                assert_eq!(res.len(), 1);
                res[0]
            }
            Err(e) => panic!("Call failed: {e}"),
        }
    }

    #[tokio::test]
    async fn test_getting_cairo0_class_from_origin_and_fork() {
        let origin_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        ));

        let json_string = std::fs::read_to_string(resolve_path(
            "../starknet-devnet-core/test_artifacts/cairo_0_test.json",
        ))
        .unwrap();
        let contract_class: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_str(&json_string).unwrap());

        // declare the contract
        let declaration_result = predeployed_account
            .declare_legacy(contract_class.clone())
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        for devnet in &[origin_devnet, fork_devnet] {
            println!("DEBUG loop"); // TODO printed only once
            let retrieved_class = devnet
                .json_rpc_client
                .get_class(BlockId::Tag(BlockTag::Latest), declaration_result.class_hash)
                .await
                .unwrap();

            assert_eq!(retrieved_class, ContractClass::Legacy(contract_class.compress().unwrap()));
        }
    }

    #[tokio::test]
    async fn test_getting_cairo1_class_from_origin_and_fork() {
        let origin_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        contract_factory
            .deploy(ctor_args.clone(), FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // generate the address of the newly deployed contract
        let contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_args,
        );

        let fork_devnet = origin_devnet.fork().await.unwrap();

        for devnet in &[origin_devnet, fork_devnet] {
            let retrieved_class_hash = devnet
                .json_rpc_client
                .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
                .await
                .unwrap();
            assert_eq!(retrieved_class_hash, declaration_result.class_hash);

            let retrieved_class = devnet
                .json_rpc_client
                .get_class(BlockId::Tag(BlockTag::Latest), declaration_result.class_hash)
                .await
                .unwrap();
            assert_cairo1_classes_equal(
                retrieved_class,
                ContractClass::Sierra(contract_class.clone()),
            )
            .unwrap();
        }
    }

    #[tokio::test]
    async fn test_forking_local_declare_deploy_fork_invoke() {
        let origin_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        contract_factory
            .deploy(ctor_args.clone(), FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // generate the address of the newly deployed contract
        let contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_args,
        );

        // assert correctly deployed
        assert_eq!(get_contract_balance(&origin_devnet, contract_address).await, initial_value);

        let fork_devnet = origin_devnet.fork().await.unwrap();

        assert_eq!(get_contract_balance(&fork_devnet, contract_address).await, initial_value);

        let fork_predeployed_account = SingleOwnerAccount::new(
            fork_devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        );

        // invoke on forked devnet
        let increment = FieldElement::from(5_u32);
        let contract_invoke = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increment, FieldElement::ZERO],
        }];

        let invoke_result = fork_predeployed_account
            .execute(contract_invoke.clone())
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // check invoke transaction
        let invoke_receipt = fork_devnet
            .json_rpc_client
            .get_transaction_receipt(invoke_result.transaction_hash)
            .await
            .unwrap();
        assert_eq!(*invoke_receipt.execution_result(), ExecutionResult::Succeeded);

        // assert origin intact and fork changed
        assert_eq!(get_contract_balance(&origin_devnet, contract_address).await, initial_value);
        assert_eq!(
            get_contract_balance(&fork_devnet, contract_address).await,
            initial_value + increment
        );
    }
}
