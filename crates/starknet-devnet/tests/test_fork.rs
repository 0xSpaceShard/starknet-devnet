pub mod common;

mod fork_tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use server::test_utils::exported_test_utils::assert_contains;
    use starknet_core::utils::get_storage_var_address;
    use starknet_rs_accounts::{
        Account, AccountFactory, AccountFactoryError, Call, ExecutionEncoding,
        OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, ContractClass, FieldElement, FunctionCall,
        MaybePendingBlockWithTxHashes, StarknetError,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_rs_signers::Signer;
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_cairo1_classes_equal, assert_tx_successful, declare_deploy,
        get_block_reader_contract_in_sierra_and_compiled_class_hash, get_json_body,
        get_simple_contract_in_sierra_and_compiled_class_hash, resolve_path,
        send_ctrl_c_signal_and_wait,
    };

    const SEPOLIA_URL: &str = "http://rpc.pathfinder.equilibrium.co/integration-sepolia/rpc/v0_7";
    const SEPOLIA_GENESIS_BLOCK_HASH: &str =
        "0x19f675d3fb226821493a6ab9a1955e384bba80f130de625621a418e9a7c0ca3";

    async fn spawn_forkable_devnet() -> Result<BackgroundDevnet, anyhow::Error> {
        let args = ["--state-archive-capacity", "full"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await?;
        Ok(devnet)
    }

    #[tokio::test]
    async fn test_fork_status() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let origin_status =
            get_json_body(origin_devnet.get("/fork_status", None).await.unwrap()).await;
        assert_eq!(origin_status, serde_json::json!({}));

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let fork_status = get_json_body(fork_devnet.get("/fork_status", None).await.unwrap()).await;
        assert_eq!(
            url::Url::from_str(fork_status["url"].as_str().unwrap()).unwrap(),
            url::Url::from_str(&origin_devnet.url).unwrap()
        );
        assert_eq!(fork_status["block"].as_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_forking_sepolia_genesis_block() {
        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", SEPOLIA_URL])
                .await
                .unwrap();

        let resp = &fork_devnet
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Hash(
                FieldElement::from_hex_be(SEPOLIA_GENESIS_BLOCK_HASH).unwrap(),
            ))
            .await;

        match resp {
            Ok(MaybePendingBlockWithTxHashes::Block(b)) => assert_eq!(b.block_number, 0),
            _ => panic!("Unexpected resp: {resp:?}"),
        };
    }

    #[tokio::test]
    async fn test_getting_non_existent_block_from_origin() {
        let fork_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--fork-network", SEPOLIA_URL])
                .await
                .expect("Could not start Devnet");

        let non_existent_block_hash = "0x123456";
        let resp = &fork_devnet
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Hash(
                FieldElement::from_hex_be(non_existent_block_hash).unwrap(),
            ))
            .await;

        match resp {
            Err(ProviderError::StarknetError(StarknetError::BlockNotFound)) => (),
            _ => panic!("Unexpected resp: {resp:?}"),
        }
    }

    #[tokio::test]
    async fn test_forking_local_genesis_block() {
        let origin_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .unwrap();
        let origin_devnet_genesis_block =
            &origin_devnet.get_latest_block_with_tx_hashes().await.unwrap();

        let block_hash = origin_devnet.create_block().await.unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let resp_block_by_hash =
            &fork_devnet.json_rpc_client.get_block_with_tx_hashes(BlockId::Hash(block_hash)).await;
        match resp_block_by_hash {
            Ok(MaybePendingBlockWithTxHashes::Block(b)) => assert_eq!(b.block_number, 1),
            _ => panic!("Unexpected resp: {resp_block_by_hash:?}"),
        };

        let resp_latest_block = &fork_devnet.get_latest_block_with_tx_hashes().await;
        match resp_latest_block {
            Ok(b) => {
                assert_eq!(b.block_number, 2);
                assert_ne!(b.block_hash, origin_devnet_genesis_block.block_hash);
            }
            _ => panic!("Unexpected resp: {resp_latest_block:?}"),
        };
    }

    #[tokio::test]
    async fn test_forked_account_balance() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        // new origin block
        let dummy_address = FieldElement::ONE;
        let mint_amount = 100;
        origin_devnet.mint(dummy_address, mint_amount).await;

        let fork_devnet = origin_devnet.fork().await.unwrap();

        // new fork block
        fork_devnet.mint(dummy_address, mint_amount).await;

        for block_i in 0..=1 {
            let block_id = BlockId::Number(block_i);
            let balance = fork_devnet.get_balance_at_block(&dummy_address, block_id).await.unwrap();
            let expected_balance = (block_i as u128 * mint_amount).into();
            assert_eq!(balance, expected_balance);
        }

        // not using get_balance_at_block=2: requires forking with --state-archive-capacity full
        let final_balance = fork_devnet.get_balance(&dummy_address, FeeUnit::WEI).await.unwrap();
        let expected_final_balance = (2_u128 * mint_amount).into();
        assert_eq!(final_balance, expected_final_balance);
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
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
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
        let _retrieved_class = fork_devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), declaration_result.class_hash)
            .await
            .unwrap();

        // assert_eq!(retrieved_class, ContractClass::Legacy(contract_class.compress().unwrap()));
        // For now, successfully unwrapping the retrieved class serves as proof of correctness.
        // Currently asserting cairo0 artifacts is failing; related: https://github.com/0xSpaceShard/starknet-devnet-rs/pull/380
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
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        let (class_hash, contract_address) =
            declare_deploy(predeployed_account, contract_class.clone(), casm_hash, &ctor_args)
                .await
                .unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let retrieved_class_hash = fork_devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();
        assert_eq!(retrieved_class_hash, class_hash);

        let expected_sierra = ContractClass::Sierra(contract_class);
        let retrieved_class_by_hash = fork_devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), class_hash)
            .await
            .unwrap();
        assert_cairo1_classes_equal(retrieved_class_by_hash, expected_sierra.clone()).unwrap();

        let retrieved_class_by_address = fork_devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();
        assert_cairo1_classes_equal(retrieved_class_by_address, expected_sierra).unwrap();
    }

    #[tokio::test]
    async fn test_origin_declare_deploy_fork_invoke() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
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
            constants::CHAIN_ID,
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

        assert_tx_successful(&invoke_result.transaction_hash, &fork_devnet.json_rpc_client).await;

        // assert origin intact and fork changed
        assert_eq!(get_contract_balance(&origin_devnet, contract_address).await, initial_value);
        assert_eq!(
            get_contract_balance(&fork_devnet, contract_address).await,
            initial_value + increment
        );
    }

    #[tokio::test]
    async fn test_deploying_account_with_class_not_present_on_origin() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let (signer, _) = origin_devnet.get_first_predeployed_account().await;

        let nonexistent_class_hash = FieldElement::from_hex_be("0x123").unwrap();
        let factory = OpenZeppelinAccountFactory::new(
            nonexistent_class_hash,
            constants::CHAIN_ID,
            signer,
            fork_devnet.clone_provider(),
        )
        .await
        .unwrap();

        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let deployment =
            factory.deploy(salt).max_fee(FieldElement::from(1e18 as u128)).send().await;
        match deployment {
            Err(AccountFactoryError::Provider(ProviderError::StarknetError(
                StarknetError::ClassHashNotFound,
            ))) => (),
            unexpected => panic!("Unexpected resp: {unexpected:?}"),
        }
    }

    #[tokio::test]
    /// For this test to make sense, origin must have a class not by default present in the fork.
    /// If https://github.com/0xSpaceShard/starknet-devnet-rs/issues/373 is addressed,
    /// both origin and fork have both of our default cairo0 and cairo1 classes, so using them for
    /// this test wouldn't make sense, as we couldn't be sure that the class used in account
    /// deployment is indeed coming from the origin.
    async fn test_deploying_account_with_class_present_on_origin() {
        let origin_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--state-archive-capacity",
            "full",
            "--account-class-custom",
            &resolve_path(
                "../starknet-devnet-core/accounts_artifacts/OpenZeppelin/0.8.0/Account.cairo/\
                 Account.sierra",
            ),
        ])
        .await
        .unwrap();

        let (signer, _) = origin_devnet.get_first_predeployed_account().await;

        // fork, but first create a forkable origin block
        origin_devnet.create_block().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();

        // deploy account using class from origin, relying on precalculated hash
        let account_hash = "0x00f7f9cd401ad39a09f095001d31f0ad3fdc2f4e532683a84a8a6c76150de858";
        let factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(account_hash).unwrap(),
            constants::CHAIN_ID,
            signer,
            fork_devnet.clone_provider(),
        )
        .await
        .unwrap();

        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let deployment = factory.deploy(salt).max_fee(FieldElement::from(1e18 as u128));
        let deployment_address = deployment.address();
        fork_devnet.mint(deployment_address, 1e18 as u128).await;
        deployment.send().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_nonce_if_contract_not_deployed() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();
        let dummy_address = FieldElement::ONE;
        match fork_devnet
            .json_rpc_client
            .get_nonce(BlockId::Tag(BlockTag::Latest), dummy_address)
            .await
        {
            Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => (),
            unexpected => panic!("Unexpected resp: {unexpected:?}"),
        }
    }

    #[tokio::test]
    async fn test_get_nonce_if_contract_deployed_on_origin() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();

        let (_, account_address) = origin_devnet.get_first_predeployed_account().await;

        let nonce = fork_devnet
            .json_rpc_client
            .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
            .await
            .unwrap();
        assert_eq!(nonce, FieldElement::ZERO);
    }

    #[tokio::test]
    async fn test_get_storage_if_contract_not_deployed() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();
        let dummy_address = FieldElement::ONE;
        let dummy_key = FieldElement::ONE;
        match fork_devnet
            .json_rpc_client
            .get_storage_at(dummy_address, dummy_key, BlockId::Tag(BlockTag::Latest))
            .await
        {
            Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => (),
            unexpected => panic!("Unexpected resp: {unexpected:?}"),
        }
    }

    #[tokio::test]
    async fn test_get_storage_if_contract_deployed_on_origin() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;

        let dummy_key = FieldElement::ONE;
        let dummy_value = fork_devnet
            .json_rpc_client
            .get_storage_at(account_address, dummy_key, BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(dummy_value, FieldElement::ZERO);

        let real_key = get_storage_var_address("Account_public_key", &[]).unwrap();
        let real_key: Felt = real_key.to_felt();
        let real_value = fork_devnet
            .json_rpc_client
            .get_storage_at::<FieldElement, FieldElement, starknet_rs_core::types::BlockId>(
                account_address,
                real_key.into(),
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();
        assert_eq!(real_value, signer.get_public_key().await.unwrap().scalar());
    }

    #[tokio::test]
    async fn test_fork_using_origin_token_contract() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let address = FieldElement::ONE;
        let mint_amount = 1000_u128;
        origin_devnet.mint(address, mint_amount).await;

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let fork_balance = fork_devnet.get_balance(&address, FeeUnit::WEI).await.unwrap();
        assert_eq!(fork_balance, FieldElement::from(mint_amount));
    }

    #[tokio::test]
    async fn test_fork_if_origin_dies() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let fork_devnet = origin_devnet.fork().await.unwrap();
        send_ctrl_c_signal_and_wait(&origin_devnet.process).await;

        let address = FieldElement::ONE;
        match fork_devnet.json_rpc_client.get_nonce(BlockId::Tag(BlockTag::Latest), address).await {
            Err(ProviderError::Other(e)) => {
                assert_contains(&e.to_string(), "error sending request")
            }
            unexpected => panic!("Got unxpected resp: {unexpected:?}"),
        }
    }

    #[tokio::test]
    async fn test_block_count_increased() {
        // latest block has number 0
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        // create two blocks
        origin_devnet.mint(0x1, 1).await; // dummy data
        let forking_block_hash = origin_devnet.create_block().await.unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        // create another block on origin - shouldn't affect fork - asserted later
        origin_devnet.create_block().await.unwrap();

        let block_after_fork =
            fork_devnet.json_rpc_client.get_block_with_tx_hashes(BlockId::Number(2)).await;

        match block_after_fork {
            Ok(MaybePendingBlockWithTxHashes::Block(b)) => {
                assert_eq!((b.block_hash, b.block_number), (forking_block_hash, 2))
            }
            _ => panic!("Unexpected resp: {block_after_fork:?}"),
        };

        let new_fork_block_hash = fork_devnet.create_block().await.unwrap();
        let new_fork_block = fork_devnet
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
            .await;

        match new_fork_block {
            Ok(MaybePendingBlockWithTxHashes::Block(b)) => {
                assert_eq!((b.block_hash, b.block_number), (new_fork_block_hash, 4));
            }
            _ => panic!("Unexpected resp: {new_fork_block:?}"),
        };
    }

    #[tokio::test]
    async fn test_block_count_increased_on_state() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();

        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            origin_devnet.clone_provider(),
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_hash) =
            get_block_reader_contract_in_sierra_and_compiled_class_hash();

        let (_, contract_address) =
            declare_deploy(predeployed_account, contract_class, casm_hash, &[]).await.unwrap();

        let fork_devnet = origin_devnet.fork().await.unwrap();

        let contract_call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_block_number").unwrap(),
            calldata: vec![],
        };
        let first_fork_block_number = fork_devnet
            .json_rpc_client
            .call(contract_call.clone(), BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(first_fork_block_number, [FieldElement::from(4_u8)]); // origin block + declare + deploy

        fork_devnet.create_block().await.unwrap();

        let second_fork_block_number = fork_devnet
            .json_rpc_client
            .call(contract_call, BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(second_fork_block_number, [FieldElement::from(5_u8)]); // origin block + declare + deploy
    }
}
