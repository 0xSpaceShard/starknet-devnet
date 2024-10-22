#![cfg(test)]
pub mod common;

mod test_v3_transactions {
    use std::sync::Arc;

    use server::test_utils::assert_contains;
    use starknet_core::constants::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, STRK_ERC20_CONTRACT_ADDRESS, UDC_CONTRACT_ADDRESS,
    };
    use starknet_rs_accounts::{
        Account, AccountDeploymentV3, AccountFactory, ConnectedAccount, DeclarationV3,
        ExecutionEncoding, ExecutionV3, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_core::types::{
        BlockId, BlockTag, Call, ExecutionResult, Felt, FlattenedSierraClass,
        InvokeTransactionResult, NonZeroFelt, StarknetError,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
    use starknet_rs_signers::LocalWallet;
    use starknet_types::felt::split_biguint;
    use starknet_types::num_bigint::BigUint;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_deployable_account_signer, get_gas_units_and_gas_price,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    enum Action {
        Declaration(Arc<FlattenedSierraClass>, Felt),
        AccountDeployment(Felt),
        Execution(Vec<Call>),
    }

    #[tokio::test]
    async fn deploy_account_to_an_address_with_insufficient_balance_should_fail() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();
        let factory = OpenZeppelinAccountFactory::new(
            Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
            constants::CHAIN_ID,
            signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        match factory.deploy_v3(Felt::THREE).send().await.unwrap_err() {
            starknet_rs_accounts::AccountFactoryError::Provider(ProviderError::StarknetError(
                StarknetError::InsufficientAccountBalance,
            )) => {}
            other => panic!("Unexpected error: {:?}", other),
        };
    }

    #[tokio::test]
    async fn declare_deploy_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let declare_transaction =
            account.declare_v3(Arc::new(sierra_artifact), casm_hash).send().await.unwrap();

        assert_tx_successful(&declare_transaction.transaction_hash, &devnet.json_rpc_client).await;

        devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), declare_transaction.class_hash)
            .await
            .unwrap();
        let salt = Felt::from_hex_unchecked("0x123");
        let constructor_arg = Felt::from(10);
        let deploy_call = vec![Call {
            to: UDC_CONTRACT_ADDRESS,
            selector: get_selector_from_name("deployContract").unwrap(),
            calldata: vec![
                declare_transaction.class_hash,
                salt,
                Felt::ZERO,      // unique
                Felt::ONE,       // constructor length
                constructor_arg, // constructor arguments
            ],
        }];

        let contract_address = get_udc_deployed_address(
            salt,
            declare_transaction.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &[constructor_arg],
        );

        let estimate_fee = account.execute_v3(deploy_call.clone()).estimate_fee().await.unwrap();
        let gas_steps = estimate_fee
            .overall_fee
            .field_div(&NonZeroFelt::try_from(estimate_fee.gas_price).unwrap());
        let deploy_transaction = account
            .execute_v3(deploy_call)
            .gas(gas_steps.to_le_digits().first().cloned().unwrap())
            .send()
            .await
            .unwrap();
        assert_tx_successful(&deploy_transaction.transaction_hash, &devnet.json_rpc_client).await;

        let class_hash_of_contract = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();

        assert_eq!(declare_transaction.class_hash, class_hash_of_contract);
    }
    #[tokio::test]
    async fn declare_from_an_account_with_insufficient_strk_tokens_balance() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (sierra_artifact, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();
        let sierra_artifact = Arc::new(sierra_artifact);
        let declaration = account.declare_v3(sierra_artifact.clone(), casm_hash);
        let estimate_fee = declaration.estimate_fee().await.unwrap();

        let account_strk_balance = devnet
            .get_balance_by_tag(&account_address, FeeUnit::FRI, BlockTag::Latest)
            .await
            .unwrap();

        // transfer balance of the account without the amount of fee

        let amount_to_transfer = account_strk_balance - estimate_fee.overall_fee + Felt::ONE;
        let amount_to_transfer = BigUint::from_bytes_le(&amount_to_transfer.to_bytes_le());

        let (high, low) = split_biguint(amount_to_transfer);

        let invoke_txn_result = account
            .execute_v3(vec![Call {
                to: STRK_ERC20_CONTRACT_ADDRESS,
                selector: get_selector_from_name("transfer").unwrap(),
                calldata: vec![
                    Felt::ONE, // recipient
                    low,       // low part of uint256
                    high,      // high part of uint256
                ],
            }])
            .send()
            .await
            .unwrap();

        assert_tx_successful(&invoke_txn_result.transaction_hash, &devnet.json_rpc_client).await;

        let account_strk_balance = devnet
            .get_balance_by_tag(&account_address, FeeUnit::FRI, BlockTag::Latest)
            .await
            .unwrap();
        assert!(estimate_fee.overall_fee > account_strk_balance);

        match declaration.send().await.unwrap_err() {
            starknet_rs_accounts::AccountError::Provider(ProviderError::StarknetError(
                StarknetError::InsufficientAccountBalance,
            )) => {}
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn invoke_with_insufficient_gas_price_and_or_gas_units_should_fail() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        transaction_with_less_gas_units_and_or_less_gas_price_should_return_error_or_be_accepted_as_reverted(
            Action::Execution(vec![Call {
                to: STRK_ERC20_CONTRACT_ADDRESS,
                selector: get_selector_from_name("transfer").unwrap(),
                calldata: vec![
                    Felt::ONE,  // recipient
                    Felt::ONE,  // low part of uint256
                    Felt::ZERO, // high part of uint256
                ],
            }]),
            Some(&account),
            Option::<&OpenZeppelinAccountFactory<LocalWallet, JsonRpcClient<HttpTransport>>>::None,
        )
        .await;
    }

    #[tokio::test]
    async fn deploy_account_with_insufficient_gas_price_and_or_gas_units_should_fail() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();
        let factory = OpenZeppelinAccountFactory::new(
            Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
            constants::CHAIN_ID,
            signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let salt = Felt::THREE;
        devnet.mint_unit(factory.deploy_v3(salt).address(), 1e18 as u128, FeeUnit::FRI).await;

        transaction_with_less_gas_units_and_or_less_gas_price_should_return_error_or_be_accepted_as_reverted(
            Action::AccountDeployment(salt),
            Option::<&SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>::None,
            Some(&factory),
        )
        .await
    }

    #[tokio::test]
    async fn declare_with_insufficient_gas_price_and_or_gas_units_should_fail() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let sierra_artifact = Arc::new(sierra_artifact);

        transaction_with_less_gas_units_and_or_less_gas_price_should_return_error_or_be_accepted_as_reverted(
            Action::Declaration(sierra_artifact, casm_hash),
            Some(&account),
            Option::<&OpenZeppelinAccountFactory<LocalWallet, JsonRpcClient<HttpTransport>>>::None,
        )
        .await;
    }

    #[tokio::test]
    async fn deploy_account_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();
        let factory = OpenZeppelinAccountFactory::new(
            Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
            constants::CHAIN_ID,
            signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let deploy_v3 = factory.deploy_v3(Felt::THREE);
        let account_address = deploy_v3.address();
        devnet.mint_unit(account_address, 1e18 as u128, FeeUnit::FRI).await;

        let fee_estimate = deploy_v3.estimate_fee().await.unwrap();

        let (gas_units, gas_price) = get_gas_units_and_gas_price(fee_estimate);
        let result = deploy_v3.gas(gas_units).gas_price(gas_price).send().await.unwrap();
        assert_tx_successful(&result.transaction_hash, &devnet.json_rpc_client).await;
    }

    /// This function sets the gas price and/or gas units to a value that is less than the estimated
    /// then sends the transaction. The expected result is that the transaction will either fail or
    /// be accepted as reverted.
    async fn transaction_with_less_gas_units_and_or_less_gas_price_should_return_error_or_be_accepted_as_reverted<
        A: ConnectedAccount + Sync,
        F: AccountFactory + Sync,
    >(
        transaction_action: Action,
        account: Option<&A>,
        account_factory: Option<&F>,
    ) {
        let estimate_fee = match &transaction_action {
            Action::Declaration(sierra_class, casm_hash) => {
                DeclarationV3::new(sierra_class.clone(), *casm_hash, account.unwrap())
                    .estimate_fee()
                    .await
                    .unwrap()
            }
            Action::AccountDeployment(salt) => {
                AccountDeploymentV3::new(*salt, account_factory.unwrap())
                    .estimate_fee()
                    .await
                    .unwrap()
            }
            Action::Execution(calls) => {
                ExecutionV3::new(calls.clone(), account.unwrap()).estimate_fee().await.unwrap()
            }
        };

        let (estimated_gas_units, gas_price) = get_gas_units_and_gas_price(estimate_fee);

        for (gas_units, gas_price) in [
            (Some(estimated_gas_units - 1), Some(gas_price)),
            (Some(estimated_gas_units), Some(gas_price - 1)),
            (Some(estimated_gas_units - 1), Some(gas_price - 1)),
            (Some(estimated_gas_units - 1), None),
            (None, Some(gas_price - 1)),
        ] {
            match &transaction_action {
                Action::Declaration(sierra_class, casm_hash) => {
                    let mut declaration =
                        DeclarationV3::new(sierra_class.clone(), *casm_hash, account.unwrap());

                    if let Some(gas_units) = gas_units {
                        declaration = declaration.gas(gas_units);
                    }
                    if let Some(gas_price) = gas_price {
                        declaration = declaration.gas_price(gas_price);
                    }
                    match declaration.send().await.unwrap_err() {
                        starknet_rs_accounts::AccountError::Provider(
                            ProviderError::StarknetError(StarknetError::InsufficientMaxFee),
                        ) => {}
                        other => panic!("Unexpected error {:?}", other),
                    }
                }
                Action::AccountDeployment(salt) => {
                    let mut account_deployment =
                        AccountDeploymentV3::new(*salt, account_factory.unwrap());
                    if let Some(gas_units) = gas_units {
                        account_deployment = account_deployment.gas(gas_units);
                    }
                    if let Some(gas_price) = gas_price {
                        account_deployment = account_deployment.gas_price(gas_price);
                    }
                    match account_deployment.send().await.unwrap_err() {
                        starknet_rs_accounts::AccountFactoryError::Provider(
                            ProviderError::StarknetError(StarknetError::InsufficientMaxFee),
                        ) => {}
                        other => panic!("Unexpected error {:?}", other),
                    }
                }
                Action::Execution(calls) => {
                    let mut execution = ExecutionV3::new(calls.clone(), account.unwrap());
                    if let Some(gas_units) = gas_units {
                        execution = execution.gas(gas_units);
                    }
                    if let Some(gas_price) = gas_price {
                        execution = execution.gas_price(gas_price);
                    }
                    let transaction_result = execution.send().await;

                    match transaction_result {
                        Ok(InvokeTransactionResult { transaction_hash }) => {
                            let receipt = account
                                .unwrap()
                                .provider()
                                .get_transaction_receipt(transaction_hash)
                                .await
                                .unwrap();
                            let execution_result = receipt.receipt.execution_result();
                            match execution_result {
                                ExecutionResult::Reverted { reason } => {
                                    assert_contains(reason.as_str(), "Insufficient max L1 gas");
                                }
                                other => panic!("Unexpected result: {:?}", other),
                            }
                        }
                        Err(starknet_rs_accounts::AccountError::Provider(
                            ProviderError::StarknetError(StarknetError::InsufficientMaxFee),
                        )) => {}
                        Err(error) => panic!("Unexpected error {:?}", error),
                    }
                }
            };
        }
    }
}
