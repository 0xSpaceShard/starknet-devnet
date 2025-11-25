use std::sync::Arc;

use starknet_rs_accounts::{
    Account, AccountDeploymentV3, AccountError, AccountFactory, ConnectedAccount, DeclarationV3,
    ExecutionEncoding, ExecutionV3, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, ExecutionResult, Felt, FlattenedSierraClass, InvokeTransactionResult,
    StarknetError, U256,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
use starknet_rs_signers::LocalWallet;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_0_ACCOUNT_CONTRACT_HASH, STRK_ERC20_CONTRACT_ADDRESS, TOO_BIG_CONTRACT_SIERRA_PATH,
    UDC_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    FeeUnit, LocalFee, assert_contains, assert_tx_succeeded_accepted, extract_json_rpc_error,
    get_deployable_account_signer, get_flattened_sierra_contract_and_casm_hash,
    get_simple_contract_artifacts,
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
        starknet_rs_accounts::AccountFactoryError::Provider(provider_error) => {
            if let Ok(json_rpc_error) = extract_json_rpc_error(provider_error) {
                if json_rpc_error.message.contains("Resources bounds")
                    || json_rpc_error.message.contains("Fee check failed")
                {
                    return;
                }
            }
            panic!("Unexpected provider error")
        }
        other => panic!("Unexpected error: {:?}", other),
    };
}

#[tokio::test]
async fn declare_deploy_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) = get_simple_contract_artifacts();

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

    assert_tx_succeeded_accepted(&declare_transaction.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

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
    let deploy_transaction = account.execute_v3(deploy_call).send().await.unwrap();
    assert_tx_succeeded_accepted(&deploy_transaction.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

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

    let (sierra_artifact, casm_hash) = get_simple_contract_artifacts();
    let sierra_artifact = Arc::new(sierra_artifact);
    let declaration = account.declare_v3(sierra_artifact.clone(), casm_hash);
    let estimate_fee = declaration.estimate_fee().await.unwrap();

    let account_strk_balance =
        devnet.get_balance_by_tag(&account_address, FeeUnit::Fri, BlockTag::Latest).await.unwrap();

    // transfer balance of the account without the amount of fee
    let amount_to_transfer =
        account_strk_balance - Felt::from(estimate_fee.overall_fee) + Felt::ONE;
    let amount_to_transfer = U256::from(amount_to_transfer);

    let invoke_txn_result = account
        .execute_v3(vec![Call {
            to: STRK_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE, // recipient
                amount_to_transfer.low().into(),
                amount_to_transfer.high().into(),
            ],
        }])
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&invoke_txn_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_strk_balance =
        devnet.get_balance_by_tag(&account_address, FeeUnit::Fri, BlockTag::Latest).await.unwrap();
    assert!(Felt::from(estimate_fee.overall_fee) > account_strk_balance);

    match declaration.send().await.unwrap_err() {
        starknet_rs_accounts::AccountError::Provider(provider_error) => {
            if let Ok(json_rpc_error) = extract_json_rpc_error(provider_error) {
                if json_rpc_error.message.contains("Resources bounds")
                    || json_rpc_error.message.contains("Fee check failed")
                {
                    return;
                }
            }
            panic!("Unexpected provider error")
        }
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
        .await.unwrap();
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
    devnet.mint_unit(factory.deploy_v3(salt).address(), 1e18 as u128, FeeUnit::Fri).await;

    transaction_with_less_gas_units_and_or_less_gas_price_should_return_error_or_be_accepted_as_reverted(
            Action::AccountDeployment(salt),
            Option::<&SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>::None,
            Some(&factory),
        )
        .await.unwrap()
}

#[tokio::test]
async fn redeclaration_has_to_fail() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) = get_simple_contract_artifacts();
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
    let declaration = account.declare_v3(sierra_artifact.clone(), casm_hash);
    let fee: LocalFee = declaration.estimate_fee().await.unwrap().into();

    declaration.send().await.unwrap();
    // redeclaration
    match declaration
        .l1_data_gas(fee.l1_data_gas)
        .l2_gas(fee.l2_gas)
        .l1_gas(fee.l1_gas)
        .send()
        .await
        .unwrap_err()
    {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::ClassAlreadyDeclared,
        )) => {}
        other => panic!("Unexpected error {:?}", other),
    }
}

#[tokio::test]
async fn declare_with_insufficient_gas_price_and_or_gas_units_should_fail() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) = get_simple_contract_artifacts();

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
        .await.unwrap();
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
    devnet.mint_unit(account_address, 1e18 as u128, FeeUnit::Fri).await;

    let result = deploy_v3.send().await.unwrap();
    assert_tx_succeeded_accepted(&result.transaction_hash, &devnet.json_rpc_client).await.unwrap();
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
) -> Result<(), anyhow::Error> {
    let estimate_fee = match &transaction_action {
        Action::Declaration(sierra_class, casm_hash) => DeclarationV3::new(
            sierra_class.clone(),
            *casm_hash,
            account.ok_or(anyhow::anyhow!("Account not found"))?,
        )
        .estimate_fee()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?,
        Action::AccountDeployment(salt) => AccountDeploymentV3::new(
            *salt,
            account_factory.ok_or(anyhow::anyhow!("Account Factory is None"))?,
        )
        .estimate_fee()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?,
        Action::Execution(calls) => {
            ExecutionV3::new(calls.clone(), account.ok_or(anyhow::anyhow!("Account not found"))?)
                .estimate_fee()
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?
        }
    };

    let LocalFee {
        l2_gas, l2_gas_price, l1_data_gas, l1_gas, l1_data_gas_price, l1_gas_price, ..
    } = LocalFee::from(estimate_fee);

    for (l2, l2_price) in [
        (l2_gas - 1, l2_gas_price),
        (l2_gas, l2_gas_price - 1),
        (l2_gas - 1, l2_gas_price - 1),
        (l2_gas - 1, 0),
        (0, l2_gas_price),
    ] {
        match &transaction_action {
            Action::Declaration(sierra_class, casm_hash) => {
                let declaration =
                    DeclarationV3::new(sierra_class.clone(), *casm_hash, account.unwrap())
                        .l1_data_gas(l1_data_gas)
                        .l1_data_gas_price(l1_data_gas_price)
                        .l1_gas(l1_gas)
                        .l1_gas_price(l1_gas_price)
                        .l2_gas(l2)
                        .l2_gas_price(l2_price);
                match declaration.send().await.unwrap_err() {
                    starknet_rs_accounts::AccountError::Provider(
                        ProviderError::StarknetError(
                            StarknetError::InsufficientResourcesForValidate,
                        ),
                    ) => {}
                    starknet_rs_accounts::AccountError::Provider(provider_error) => {
                        if let Ok(json_rpc_error) = extract_json_rpc_error(provider_error) {
                            if json_rpc_error.message.contains("Resource bounds were not satisfied")
                            {
                                return Ok(());
                            }
                        }
                        anyhow::bail!("Unexpected provider error")
                    }
                    other => anyhow::bail!("Unexpected error {:?}", other),
                }
            }
            Action::AccountDeployment(salt) => {
                let account_deployment = AccountDeploymentV3::new(*salt, account_factory.unwrap())
                    .l1_data_gas(l1_data_gas)
                    .l1_data_gas_price(l1_data_gas_price)
                    .l1_gas(l1_gas)
                    .l1_gas_price(l1_gas_price)
                    .l2_gas(l2)
                    .l2_gas_price(l2_price);
                match account_deployment.send().await.unwrap_err() {
                    starknet_rs_accounts::AccountFactoryError::Provider(
                        ProviderError::StarknetError(
                            StarknetError::InsufficientResourcesForValidate,
                        ),
                    ) => {}
                    starknet_rs_accounts::AccountFactoryError::Provider(provider_error) => {
                        println!("Provider error: {:?}", provider_error);
                        if let Ok(json_rpc_error) = extract_json_rpc_error(provider_error) {
                            println!("JSON RPC Error: {:?}", json_rpc_error);
                            if json_rpc_error.message.contains("Resource bounds were not satisfied")
                            {
                                return Ok(());
                            }
                        }
                        anyhow::bail!("Unexpected provider error")
                    }
                    other => anyhow::bail!("Unexpected error {:?}", other),
                }
            }
            Action::Execution(calls) => {
                let execution = ExecutionV3::new(calls.clone(), account.unwrap())
                    .l1_data_gas(l1_data_gas)
                    .l1_data_gas_price(l1_data_gas_price)
                    .l1_gas(l1_gas)
                    .l1_gas_price(l1_gas_price)
                    .l2_gas(l2)
                    .l2_gas_price(l2_price);
                let transaction_result = execution.send().await;

                match transaction_result {
                    Ok(InvokeTransactionResult { transaction_hash }) => {
                        let receipt = account
                            .ok_or(anyhow::anyhow!("Account not found"))?
                            .provider()
                            .get_transaction_receipt(transaction_hash)
                            .await?;
                        let execution_result = receipt.receipt.execution_result();
                        match execution_result {
                            ExecutionResult::Reverted { reason } => {
                                assert_contains(reason.as_str(), "Insufficient max L2Gas").unwrap();
                            }
                            other => anyhow::bail!("Unexpected result: {:?}", other),
                        }
                    }
                    Err(starknet_rs_accounts::AccountError::Provider(provider_error)) => {
                        if let Ok(json_rpc_error) = extract_json_rpc_error(provider_error) {
                            if json_rpc_error.message.contains("Resource bounds were not satisfied")
                            {
                                return Ok(());
                            }
                        }
                        anyhow::bail!("Unexpected provider error")
                    }
                    Err(error) => anyhow::bail!("Unexpected error {:?}", error),
                }
            }
        };
    }

    Ok(())
}

#[tokio::test]
async fn test_rejection_of_too_big_class_declaration() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(TOO_BIG_CONTRACT_SIERRA_PATH);

    match account.declare_v3(Arc::new(contract_class), casm_hash).send().await {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::ContractClassSizeIsTooLarge,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    }
}
