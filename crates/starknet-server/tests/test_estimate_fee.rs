pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::constants::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, QUERY_VERSION_BASE, UDC_CONTRACT_ADDRESS,
    };
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{
        Account, AccountError, AccountFactory, AccountFactoryError, Call, ConnectedAccount,
        ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, BroadcastedInvokeTransaction,
        BroadcastedTransaction, FeeEstimate, FieldElement, FunctionCall, StarknetError,
    };
    use starknet_rs_core::utils::{
        cairo_short_string_to_felt, get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CAIRO_1_CONTRACT_PATH, CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH,
        CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID,
    };
    use crate::common::utils::{
        assert_tx_reverted, assert_tx_successful, get_deployable_account_signer,
        get_flattened_sierra_contract_and_casm_hash,
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
        let unsuccessful_deployment_tx = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from((fee_estimation.overall_fee - 1) as u128))
            .nonce(new_account_nonce)
            .send()
            .await;
        match unsuccessful_deployment_tx {
            Err(AccountFactoryError::Provider(ProviderError::StarknetError(
                StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::InsufficientMaxFee),
                    ..
                },
            ))) => (),
            other => panic!("Unexpected result: {other:?}"),
        };

        // try sending with sufficient max fee
        let successful_deployment = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128))
            .nonce(new_account_nonce)
            .send()
            .await
            .expect("Should deploy with sufficient fee");
        assert_tx_successful(&successful_deployment.transaction_hash, &devnet.json_rpc_client)
            .await;
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
        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        // get class
        let contract_artifact = dummy_cairo_0_contract_class();
        let contract_artifact = Arc::new(serde_json::from_value(contract_artifact.inner).unwrap());

        // declare class
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        );

        let fee_estimation = account
            .declare_legacy(Arc::clone(&contract_artifact))
            .nonce(FieldElement::ZERO)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // try sending with insufficient max fee
        let unsuccessful_declare_tx = account
            .declare_legacy(Arc::clone(&contract_artifact))
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from((fee_estimation.overall_fee - 1) as u128))
            .send()
            .await;
        match unsuccessful_declare_tx {
            Err(AccountError::Provider(ProviderError::StarknetError(
                StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::InsufficientMaxFee),
                    ..
                },
            ))) => (),
            other => panic!("Unexpected result: {other:?}"),
        };

        // try sending with sufficient max fee
        let successful_declare_tx = account
            .declare_legacy(contract_artifact)
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128))
            .send()
            .await
            .unwrap();
        assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client)
            .await;
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v2() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);
        let flattened_contract_artifact = Arc::new(flattened_contract_artifact);

        // declare class
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        );

        let fee_estimation = account
            .declare(Arc::clone(&flattened_contract_artifact), casm_hash)
            .nonce(FieldElement::ZERO)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // try sending with insufficient max fee
        let unsuccessful_declare_tx = account
            .declare(Arc::clone(&flattened_contract_artifact), casm_hash)
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from((fee_estimation.overall_fee - 1) as u128))
            .send()
            .await;
        match unsuccessful_declare_tx {
            Err(AccountError::Provider(ProviderError::StarknetError(
                StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::InsufficientMaxFee),
                    ..
                },
            ))) => (),
            other => panic!("Unexpected result: {other:?}"),
        };

        // try sending with sufficient max fee
        let successful_declare_tx = account
            .declare(Arc::clone(&flattened_contract_artifact), casm_hash)
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128))
            .send()
            .await
            .unwrap();
        assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client)
            .await;
    }

    #[tokio::test]
    async fn estimate_fee_of_invoke() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        ));

        // get class
        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
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

        // invoke with insufficient max_fee
        let insufficient_max_fee = FieldElement::from((fee_estimation.overall_fee - 1) as u128);
        let unsuccessful_invoke_tx = account
            .execute(invoke_calls.clone())
            .max_fee(insufficient_max_fee)
            .send()
            .await
            .unwrap();
        let balance_after_insufficient = devnet
            .json_rpc_client
            .call(call.clone(), BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(balance_after_insufficient, vec![FieldElement::ZERO]);

        assert_tx_reverted(
            &unsuccessful_invoke_tx.transaction_hash,
            &devnet.json_rpc_client,
            &["Calculated fee", "exceeds max fee"],
        )
        .await;

        // invoke with sufficient max_fee
        let sufficient_max_fee =
            FieldElement::from((fee_estimation.overall_fee as f64 * 1.1) as u128);
        account.execute(invoke_calls).max_fee(sufficient_max_fee).send().await.unwrap();
        let balance_after_sufficient =
            devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
        assert_eq!(balance_after_sufficient, vec![increase_amount]);
    }

    #[tokio::test]
    async fn message_available_if_estimation_panics() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        ));

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
        let class_hash = flattened_contract_artifact.class_hash();

        // declare class
        let declaration_result =
            account.declare(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
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
            .send()
            .await
            .expect("Cannot deploy");

        let panic_reason = "custom little reason";
        let calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("create_panic").unwrap(),
            calldata: vec![cairo_short_string_to_felt(panic_reason).unwrap()],
        }];

        let prepared = account
            .execute(calls.clone())
            .nonce(account.get_nonce().await.unwrap())
            .max_fee(FieldElement::ZERO)
            .prepared()
            .unwrap()
            .get_invoke_request(true)
            .await
            .unwrap();

        let params = json!({
            "block_id": "latest",
            "request": [
                serde_json::to_value(prepared).unwrap()
            ]
        });

        let result = devnet.send_custom_rpc("starknet_estimateFee", params).await;
        let revert_error = result["error"]["data"]["revert_error"].as_str().unwrap();

        assert!(revert_error.contains(panic_reason));
    }

    #[tokio::test]
    async fn using_query_version_if_estimating() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        ));

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_VERSION_ASSERTER_SIERRA_PATH);
        let class_hash = flattened_contract_artifact.class_hash();

        // declare class
        let declaration_result =
            account.declare(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
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
            .send()
            .await
            .expect("Cannot deploy");

        let expected_version = QUERY_VERSION_BASE + FieldElement::ONE;
        let calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("assert_version").unwrap(),
            calldata: vec![expected_version],
        }];

        match account.execute(calls).estimate_fee().await {
            Ok(_) => (),
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    /// estimate fee of declare + deploy (invoke udc)
    async fn estimate_fee_of_multiple_txs() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        // get class
        let contract_json = dummy_cairo_0_contract_class();
        let contract_class: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
        let class_hash = contract_class.class_hash().unwrap();

        let deployment_selector =
            format!("{:x}", get_selector_from_name("deployContract").unwrap());

        // precalculated signatures
        let declaration_signature = [
            "0x419d7466b316867092abdc63556471002a94077f67b929d14aaec7e2f367de8",
            "0x619b4ee80392a8d11e44de3bd9919a2db870a55f6dfe0c1eb6aefaf947bf3a7",
        ];
        let deployment_signature = [
            "0x30311fbdb604cb08e54e7cc3ab0a4442e30a6f637d440d9e3c6590cc827a183",
            "0x650acb8c4d9f1041cb20a078f5c7afcdfacd2333c3f9774c8cf2ea043246316",
        ];

        devnet
            .json_rpc_client
            .estimate_fee(
                [
                    BroadcastedTransaction::Declare(
                        starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                            BroadcastedDeclareTransactionV1 {
                                max_fee: FieldElement::ZERO,
                                signature: declaration_signature
                                    .into_iter()
                                    .map(|s| FieldElement::from_hex_be(s).unwrap())
                                    .collect(),
                                nonce: FieldElement::ZERO,
                                sender_address: account_address,
                                contract_class: contract_class.compress().unwrap().into(),
                                is_query: false,
                            },
                        ),
                    ),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction {
                        max_fee: FieldElement::ZERO,
                        // precalculated signature
                        signature: deployment_signature
                            .into_iter()
                            .map(|s| FieldElement::from_hex_be(s).unwrap())
                            .collect(),
                        nonce: FieldElement::ONE,
                        sender_address: account_address,
                        calldata: [
                            "0x1",
                            UDC_CONTRACT_ADDRESS,
                            deployment_selector.as_str(),
                            "0x0",
                            "0x4",
                            "0x4",
                            format!("{:x}", class_hash).as_str(),
                            "0x123", // salt
                            "0x0",
                            "0x0",
                        ]
                        .into_iter()
                        .map(|s| FieldElement::from_hex_be(s).unwrap())
                        .collect(),
                        is_query: false,
                    }),
                ],
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap()
            .iter()
            .for_each(assert_fee_estimation);
    }
}
