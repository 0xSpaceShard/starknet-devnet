use std::sync::Arc;

use starknet_core::constants::ENTRYPOINT_NOT_FOUND_ERROR_ENCODED;
use starknet_rs_accounts::{
    Account, AccountError, AccountFactory, AccountFactoryError, ConnectedAccount, ExecutionEncoder,
    ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    BlockId, BlockTag, BroadcastedDeclareTransactionV3, BroadcastedInvokeTransactionV3,
    BroadcastedTransaction, Call, DataAvailabilityMode, FeeEstimate, Felt, FunctionCall,
    ResourceBounds, ResourceBoundsMapping, StarknetError, TransactionExecutionErrorData,
};
use starknet_rs_core::utils::{
    UdcUniqueness, cairo_short_string_to_felt, get_selector_from_name, get_udc_deployed_address,
};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
use starknet_rs_signers::{LocalWallet, Signer};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_CONTRACT_PATH, CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH,
    CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID, ETH_ERC20_CONTRACT_ADDRESS,
    QUERY_VERSION_OFFSET, UDC_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    LocalFee, assert_contains, assert_tx_reverted, assert_tx_successful, extract_message_error,
    extract_nested_error, get_deployable_account_signer,
    get_flattened_sierra_contract_and_casm_hash, get_simple_contract_artifacts,
};

fn assert_fee_estimation(fee_estimation: &FeeEstimate) {
    assert_eq!(
        fee_estimation.l1_data_gas_consumed * fee_estimation.l1_data_gas_price
            + fee_estimation.l2_gas_consumed * fee_estimation.l2_gas_price
            + fee_estimation.l1_gas_consumed * fee_estimation.l1_gas_price,
        fee_estimation.overall_fee
    );
    assert!(fee_estimation.overall_fee > Felt::ZERO, "Checking fee_estimation: {fee_estimation:?}");
}

#[tokio::test]
async fn estimate_fee_of_deploy_account() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // define the key of the new account - dummy value
    let new_account_signer = get_deployable_account_signer();
    let account_factory = OpenZeppelinAccountFactory::new(
        Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
        CHAIN_ID,
        new_account_signer.clone(),
        devnet.clone_provider(),
    )
    .await
    .unwrap();
    let new_account_nonce = Felt::ZERO;

    // fund address
    let salt = Felt::from_hex_unchecked("0x123");
    let deployment = account_factory.deploy_v3(salt);
    let deployment_address = deployment.address();
    let fee_estimation = account_factory
        .deploy_v3(salt)
        .gas_estimate_multiplier(1.0)
        .gas_price_estimate_multiplier(1.0)
        .nonce(new_account_nonce)
        .estimate_fee()
        .await
        .unwrap();
    assert_fee_estimation(&fee_estimation);

    // fund the account before deployment
    let mint_amount = fee_estimation.overall_fee * Felt::TWO;
    devnet.mint(deployment_address, mint_amount.to_biguint().try_into().unwrap()).await;
    let fee = LocalFee::from(fee_estimation);
    // try sending with insufficient resource bounds
    let unsuccessful_deployment_tx = account_factory
        .deploy_v3(salt)
        .l1_data_gas(fee.l1_data_gas)
        .l1_gas(fee.l1_gas)
        .l2_gas(fee.l2_gas - 1)
        .nonce(new_account_nonce)
        .send()
        .await;
    match unsuccessful_deployment_tx {
        Err(AccountFactoryError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientResourcesForValidate,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    // try sending with sufficient gas bounds
    let successful_deployment = account_factory
        .deploy_v3(salt)
        .gas_estimate_multiplier(1.1)
        .gas_price_estimate_multiplier(1.1)
        .nonce(new_account_nonce)
        .send()
        .await
        .expect("Should deploy with sufficient fee");
    assert_tx_successful(&successful_deployment.transaction_hash, &devnet.json_rpc_client).await;
}

#[tokio::test]
async fn estimate_fee_of_invalid_deploy_account() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let new_account_signer = get_deployable_account_signer();
    let invalid_class_hash = Felt::from_hex_unchecked("0x123");
    let account_factory = OpenZeppelinAccountFactory::new(
        invalid_class_hash,
        CHAIN_ID,
        new_account_signer,
        devnet.clone_provider(),
    )
    .await
    .unwrap();

    let salt = Felt::from_hex_unchecked("0x123");
    let deployment = account_factory.deploy_v3(salt);
    match deployment.estimate_fee().await {
        Err(AccountFactoryError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(err),
        ))) if err.transaction_index == 0 => {
            assert_contains(
                &format!("{:?}", err.execution_error),
                &format!(
                    "Class with hash {} is not declared.",
                    invalid_class_hash.to_fixed_hex_string()
                ),
            );
        }
        other => panic!("Unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn estimate_fee_of_declare_v3() {
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
        ExecutionEncoding::New,
    );

    // two times in a row should produce the same result
    let mut fee_estimations: Vec<FeeEstimate> = vec![];
    for _ in 0..2 {
        let fee_estimation = account
            .declare_v3(Arc::clone(&flattened_contract_artifact), casm_hash)
            .nonce(Felt::ZERO)
            .gas_estimate_multiplier(1.0)
            .gas_price_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);
        fee_estimations.push(fee_estimation);
    }
    assert_eq!(fee_estimations[0], fee_estimations[1]);
    let fee_estimation = &fee_estimations[0];
    let fee = LocalFee::from(fee_estimation.clone());
    // try sending with insufficient resource bounds
    let unsuccessful_declare_tx = account
        .declare_v3(Arc::clone(&flattened_contract_artifact), casm_hash)
        .nonce(Felt::ZERO)
        .l1_gas(fee.l1_gas)
        .l1_data_gas(fee.l1_data_gas)
        .l2_gas(fee.l2_gas - 1)
        .send()
        .await;
    match unsuccessful_declare_tx {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientResourcesForValidate,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    // try sending with sufficient gas bounds
    let successful_declare_tx = account
        .declare_v3(Arc::clone(&flattened_contract_artifact), casm_hash)
        .nonce(Felt::ZERO)
        .l1_gas((fee.l1_gas as f64 * 1.1) as u64)
        .l1_data_gas((fee.l1_data_gas as f64 * 1.1) as u64)
        .l2_gas((fee.l2_gas as f64 * 1.1) as u64)
        .send()
        .await
        .unwrap();
    assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client).await;
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
        ExecutionEncoding::New,
    ));

    // get class
    let (contract_artifact, casm_hash) = get_simple_contract_artifacts();
    let contract_artifact = Arc::new(contract_artifact);
    let class_hash = contract_artifact.class_hash();

    // declare class
    let declaration_result =
        account.declare_v3(contract_artifact, casm_hash).nonce(Felt::ZERO).send().await.unwrap();
    assert_eq!(declaration_result.class_hash, class_hash);

    // deploy instance of class
    let contract_factory = ContractFactory::new(class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![Felt::ZERO];
    let contract_address = get_udc_deployed_address(
        salt,
        class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v3(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    // prepare the call used in estimation and actual invoke
    let invoke_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![Felt::from(100_u128), Felt::ONE], // increment amount
    }];

    // estimate the fee
    let fee_estimation = account.execute_v3(invoke_calls.clone()).estimate_fee().await.unwrap();
    assert_fee_estimation(&fee_estimation);

    // prepare the call used in checking the balance
    let call = FunctionCall {
        contract_address,
        entry_point_selector: get_selector_from_name("get_balance").unwrap(),
        calldata: vec![],
    };

    // invoke with insufficient resource bounds
    let unsuccessful_invoke_tx = account
        .execute_v3(invoke_calls.clone())
        .gas_estimate_multiplier(0.9)
        .gas_price_estimate_multiplier(1.0)
        .send()
        .await
        .unwrap();
    let balance_after_insufficient =
        devnet.json_rpc_client.call(call.clone(), BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(balance_after_insufficient, vec![Felt::ZERO]);

    assert_tx_reverted(
        &unsuccessful_invoke_tx.transaction_hash,
        &devnet.json_rpc_client,
        &["Insufficient max L2Gas"],
    )
    .await;

    // invoke with sufficient resource bounds
    let fee = LocalFee::from(fee_estimation);
    account
        .execute_v3(invoke_calls)
        .l1_gas(fee.l1_gas)
        .l1_data_gas(fee.l1_data_gas)
        .l2_gas(fee.l2_gas)
        .send()
        .await
        .unwrap();
    let balance_after_sufficient =
        devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(balance_after_sufficient, vec![Felt::from(101_u128)]);
}

#[tokio::test]
async fn message_available_if_estimation_reverts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    // declare class
    let declaration_result =
        account.declare_v3(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
    assert_eq!(declaration_result.class_hash, class_hash);

    // deploy instance of class
    let contract_factory = ContractFactory::new(class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![];
    let contract_address = get_udc_deployed_address(
        salt,
        class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v3(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    let panic_reason = cairo_short_string_to_felt("custom little reason").unwrap();
    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("create_panic").unwrap(),
        calldata: vec![panic_reason],
    }];

    let invoke_err = account
        .execute_v3(calls.clone())
        .nonce(account.get_nonce().await.unwrap())
        .estimate_fee()
        .await
        .unwrap_err();

    match invoke_err {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                transaction_index: 0,
                execution_error,
                ..
            }),
        )) => {
            let account_error = extract_nested_error(&execution_error);
            let contract_error = extract_nested_error(&account_error.error);
            let inner_error = extract_nested_error(&contract_error.error);
            let error_msg = extract_message_error(&inner_error.error);
            assert_contains(error_msg, &panic_reason.to_hex_string());
        }
        other => panic!("Invalid err: {other:?}"),
    };
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
        ExecutionEncoding::New,
    ));

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_VERSION_ASSERTER_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    // declare class
    let declaration_result =
        account.declare_v3(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
    assert_eq!(declaration_result.class_hash, class_hash);

    // deploy instance of class
    let contract_factory = ContractFactory::new(class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![];
    let contract_address = get_udc_deployed_address(
        salt,
        class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v3(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    let expected_version = QUERY_VERSION_OFFSET + Felt::THREE;
    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("assert_version").unwrap(),
        calldata: vec![expected_version],
    }];

    match account.execute_v3(calls).estimate_fee().await {
        Ok(_) => (),
        other => panic!("Unexpected result: {other:?}"),
    }
}

async fn broadcasted_invoke_v3_for_estimation(
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    signer: &LocalWallet,
    to_address: Felt,
    selector: Felt,
    calldata: &[Felt],
    nonce: Felt,
) -> Result<BroadcastedTransaction, anyhow::Error> {
    let calls = vec![Call { to: to_address, selector, calldata: calldata.to_vec() }];
    let calldata = account.encode_calls(&calls);

    let execution_v3 = account.execute_v3(calls);

    let l1_gas_consumed = 0;
    let l1_gas_price = 0;
    let l1_data_gas_consumed = 0;
    let l1_data_gas_price = 0;
    let l2_gas_consumed = 0;
    let l2_gas_price = 0;

    let prepared_invoke = execution_v3
        .nonce(nonce)
        .l1_gas(l1_gas_consumed)
        .l2_gas(l2_gas_consumed)
        .l1_data_gas(l1_data_gas_consumed)
        .l1_gas_price(l1_gas_price)
        .l1_data_gas_price(l1_data_gas_price)
        .l2_gas_price(l2_gas_price)
        .prepared()?;

    let is_query = false;
    let signature = signer.sign_hash(&prepared_invoke.transaction_hash(is_query)).await?;

    Ok(BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
        resource_bounds: ResourceBoundsMapping {
            l1_gas: ResourceBounds {
                max_amount: l1_gas_consumed,
                max_price_per_unit: l1_gas_price,
            },
            l1_data_gas: ResourceBounds {
                max_amount: l1_data_gas_consumed,
                max_price_per_unit: l1_data_gas_price,
            },
            l2_gas: ResourceBounds {
                max_amount: l2_gas_consumed,
                max_price_per_unit: l2_gas_price,
            },
        },
        signature: vec![signature.r, signature.s],
        nonce,
        sender_address: account.address(),
        calldata,
        is_query,
        tip: 0,
        paymaster_data: vec![],
        account_deployment_data: vec![],
        nonce_data_availability_mode: DataAvailabilityMode::L1,
        fee_data_availability_mode: DataAvailabilityMode::L1,
    }))
}

#[tokio::test]
/// estimate fee of declare + deploy (invoke udc)
async fn estimate_fee_of_multiple_txs() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo0"])
        .await
        .expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::Legacy,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    // get class
    let (contract_class, casm_hash) = get_simple_contract_artifacts();
    let contract_class = Arc::new(contract_class);
    let class_hash = contract_class.class_hash();

    let declaration_nonce = Felt::ZERO;
    let l1_gas = 0;
    let l1_gas_price = 0;
    let prepared_declaration = account
        .declare_v3(contract_class.clone(), casm_hash)
        .l1_gas(l1_gas)
        .l1_gas_price(l1_gas_price)
        .l2_gas(0)
        .l2_gas_price(0)
        .l1_data_gas(0)
        .l1_data_gas_price(0)
        .nonce(declaration_nonce)
        .prepared()
        .unwrap();

    let query_only = false;
    let declaration_signature =
        signer.sign_hash(&prepared_declaration.transaction_hash(query_only)).await.unwrap();

    devnet
        .json_rpc_client
        .estimate_fee(
            [
                BroadcastedTransaction::Declare(BroadcastedDeclareTransactionV3 {
                    signature: vec![declaration_signature.r, declaration_signature.s],
                    nonce: declaration_nonce,
                    sender_address: account_address,
                    contract_class,
                    is_query: query_only,
                    compiled_class_hash: casm_hash,
                    resource_bounds: ResourceBoundsMapping {
                        l1_gas: ResourceBounds {
                            max_amount: l1_gas,
                            max_price_per_unit: l1_gas_price,
                        },
                        l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                        l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                    },
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode: DataAvailabilityMode::L1,
                    fee_data_availability_mode: DataAvailabilityMode::L1,
                }),
                broadcasted_invoke_v3_for_estimation(
                    &account,
                    &signer,
                    UDC_CONTRACT_ADDRESS,
                    get_selector_from_name("deployContract").unwrap(),
                    &[
                        class_hash,
                        Felt::from_hex_unchecked("0x123"), // salt
                        Felt::ZERO,                        // unique
                        Felt::ONE,                         // ctor args len
                        Felt::ZERO,                        // ctor args - [initial_balance]
                    ],
                    Felt::ONE,
                )
                .await
                .unwrap(),
            ],
            [], // simulation_flags
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .unwrap()
        .iter()
        .for_each(assert_fee_estimation);
}

#[tokio::test]
async fn estimate_fee_of_multiple_txs_with_second_failing() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    );

    let non_existent_selector = get_selector_from_name("nonExistentMethod").unwrap();

    let err = devnet
        .json_rpc_client
        .estimate_fee(
            [
                broadcasted_invoke_v3_for_estimation(
                    &account,
                    &signer,
                    ETH_ERC20_CONTRACT_ADDRESS,
                    get_selector_from_name("transfer").unwrap(),
                    &[
                        Felt::ONE,                 // recipient
                        Felt::from(1_000_000_000), // low part of uint256
                        Felt::ZERO,                // high part of uint256
                    ],
                    Felt::ZERO, // original nonce
                )
                .await
                .unwrap(),
                broadcasted_invoke_v3_for_estimation(
                    &account,
                    &signer,
                    ETH_ERC20_CONTRACT_ADDRESS,
                    non_existent_selector,
                    &[],
                    Felt::ONE, // nonce incremented after 1st tx
                )
                .await
                .unwrap(),
            ],
            [], // simulation_flags
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .unwrap_err();

    match err {
        ProviderError::StarknetError(StarknetError::TransactionExecutionError(
            TransactionExecutionErrorData { transaction_index, execution_error },
        )) => {
            assert_eq!(transaction_index, 1);
            assert_contains(
                &format!("{:?}", execution_error),
                &ENTRYPOINT_NOT_FOUND_ERROR_ENCODED.to_hex_string(),
            );
        }
        _ => panic!("Unexpected error: {err}"),
    };
}

#[tokio::test]
async fn estimate_fee_of_multiple_failing_txs_should_return_index_of_the_first_failing_transaction()
{
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::New,
    );

    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    let estimate_fee_resource_bounds = ResourceBoundsMapping {
        l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
    };

    // call non existent method in UDC
    let calls = vec![Call {
        to: UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("no_such_method").unwrap(),
        calldata: vec![
            class_hash,
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,
            Felt::ZERO,
        ],
    }];

    let calldata = account.encode_calls(&calls);

    let is_query = true;
    let nonce_data_availability_mode = DataAvailabilityMode::L1;
    let fee_data_availability_mode = DataAvailabilityMode::L1;

    let expected_error = devnet
        .json_rpc_client
        .estimate_fee(
            [
                BroadcastedTransaction::Declare(BroadcastedDeclareTransactionV3 {
                    sender_address: account_address,
                    compiled_class_hash: casm_hash,
                    signature: vec![],
                    nonce: Felt::ZERO,
                    contract_class: Arc::new(flattened_contract_artifact.clone()),
                    resource_bounds: estimate_fee_resource_bounds.clone(),
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
                    sender_address: account_address,
                    calldata,
                    signature: vec![],
                    nonce: Felt::ONE,
                    resource_bounds: estimate_fee_resource_bounds.clone(),
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
            ],
            [],
            account.block_id(),
        )
        .await
        .unwrap_err();

    match expected_error {
        ProviderError::StarknetError(StarknetError::TransactionExecutionError(
            TransactionExecutionErrorData { transaction_index, execution_error },
        )) => {
            assert_eq!(transaction_index, 0);
            assert_contains(&format!("{:?}", execution_error), "invalid signature");
        }
        other => panic!("Unexpected error: {:?}", other),
    }
}
