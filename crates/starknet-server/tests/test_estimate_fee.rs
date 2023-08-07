pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_accounts::{
        Account, AccountFactory, AccountFactoryError, Call, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::contract::SierraClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, FeeEstimate, FieldElement, FunctionCall, StarknetError,
    };
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::constants::{
        CAIRO_0_CONTRACT_PATH, CAIRO_1_CONTRACT_PATH, CASM_COMPILED_CLASS_HASH, CHAIN_ID,
    };
    use crate::common::util::{
        get_deployable_account_signer, get_predeployed_account_props, load_json,
        resolve_crates_path, BackgroundDevnet,
    };

    fn assert_fee_estimation(fee_estimation: &FeeEstimate) {
        assert_eq!(
            fee_estimation.gas_price * fee_estimation.gas_consumed,
            fee_estimation.overall_fee
        );
        assert!(fee_estimation.overall_fee > 0u64, "Checking fee_estimation: {fee_estimation:?}");
    }

    #[tokio::test]
    async fn estimate_fee_of_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;

        // fund address
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let deployment = account_factory.deploy(salt);
        let deployment_address = deployment.address();
        let fee_estimation = account_factory
            .deploy(salt)
            .fee_estimate_multiplier(1.0)
            .nonce(new_account_nonce)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // fund the account before deployment
        let mint_amount = fee_estimation.overall_fee as u128 * 2;
        devnet.mint(deployment_address, mint_amount).await;

        // try sending with insufficient max fee
        // TODO uncomment the following section once starknet_in_rust fixes max_fee checking
        // let unsuccessful_deployment_tx = account_factory
        //     .deploy(salt)
        //     .max_fee(FieldElement::from((fee_estimation.overall_fee as f64 * 0.9) as u128))
        //     .nonce(new_account_nonce)
        //     .send()
        //     .await
        //     .unwrap();
        // todo!("Assert the tx is not accepted");

        // try sending with sufficient max fee
        let _result = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128))
            .nonce(new_account_nonce)
            .send()
            .await
            .expect("Should deploy with sufficient fee");
        // TODO assert tx is accepted
    }

    #[tokio::test]
    async fn estimate_fee_of_invalid_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let new_account_signer = get_deployable_account_signer();
        let dummy_invalid_class_hash = FieldElement::from_hex_be("0x123").unwrap();
        let account_factory = OpenZeppelinAccountFactory::new(
            dummy_invalid_class_hash,
            CHAIN_ID,
            new_account_signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;

        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let err = account_factory
            .deploy(salt)
            .nonce(new_account_nonce)
            .estimate_fee()
            .await
            .expect_err("Should have failed");
        match err {
            AccountFactoryError::Provider(ProviderError::StarknetError(
                StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::ContractError),
                    ..
                },
            )) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();

        // get class
        let contract_artifact_path = resolve_crates_path(CAIRO_0_CONTRACT_PATH);
        let contract_artifact: LegacyContractClass = load_json(&contract_artifact_path);

        // declare class
        let account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, account_address, CHAIN_ID);

        let fee_estimation = account
            .declare_legacy(Arc::new(contract_artifact))
            .nonce(FieldElement::ZERO)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // TODO attempt declaring with max_fee < estimate - expect failure
        // TODO attempt declaring with max_fee > estimate - expect success
    }

    #[ignore] // estimation currently completely failing
    #[tokio::test]
    async fn estimate_fee_of_declare_v2() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();

        // get class
        let contract_artifact_path = resolve_crates_path(CAIRO_1_CONTRACT_PATH);
        let contract_artifact: SierraClass = load_json(&contract_artifact_path);
        let flattened_contract_artifact = contract_artifact.flatten().unwrap();
        let compiled_class_hash = FieldElement::from_hex_be(CASM_COMPILED_CLASS_HASH).unwrap();

        // declare class
        let account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, account_address, CHAIN_ID);

        let fee_estimation = account
            .declare(Arc::new(flattened_contract_artifact), compiled_class_hash)
            .nonce(FieldElement::ONE)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);
        // TODO attempt declaring with max_fee < estimate - expect failure
        // TODO attempt declaring with max_fee > estimate - expect success
    }

    #[tokio::test]
    async fn estimate_fee_of_invoke() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
        ));

        // get class
        let contract_artifact_path = resolve_crates_path(CAIRO_0_CONTRACT_PATH);
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(load_json(&contract_artifact_path));
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        let declaration_result = account
            .declare_legacy(contract_artifact.clone())
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();
        assert_eq!(declaration_result.class_hash, class_hash);

        // deploy instance of class
        let contract_factory = ContractFactory::new(class_hash, account.clone());
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory
            .deploy(constructor_calldata, salt, false)
            .nonce(FieldElement::ONE)
            // max fee implicitly estimated
            .send()
            .await
            .expect("Cannot deploy");

        // prepare the call used in estimation and actual invoke
        let increase_amount = FieldElement::from(100u128);
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increase_amount],
        }];

        // estimate the fee
        let fee_estimation = account
            .execute(invoke_calls.clone())
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // prepare the call used in checking the balance
        let call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_balance").unwrap(),
            calldata: vec![],
        };

        // invoke with max_fee < estimate; expect failure
        // TODO uncomment the following section once starknet_in_rust fixes max_fee checking
        // let insufficient_max_fee =
        //     FieldElement::from((fee_estimation.overall_fee as f64 * 0.9) as u128);
        // account.execute(invoke_calls.clone()).max_fee(insufficient_max_fee).send().await.
        // unwrap(); let balance_after_insufficient = devnet
        //     .json_rpc_client
        //     .call(call.clone(), BlockId::Tag(BlockTag::Latest))
        //     .await
        //     .unwrap();
        // println!("Balance after insufficient: {balance_after_insufficient:?}");
        // assert_eq!(balance_after_insufficient, vec![FieldElement::ZERO]);

        // invoke with max_fee > estimate; expect success
        let sufficient_max_fee =
            FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128);
        account.execute(invoke_calls).max_fee(sufficient_max_fee).send().await.unwrap();
        let balance_after_sufficient =
            devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
        assert_eq!(balance_after_sufficient, vec![increase_amount]);
    }

    #[tokio::test]
    async fn estimate_fee_of_multiple_txs() {
        todo!();
    }
}
