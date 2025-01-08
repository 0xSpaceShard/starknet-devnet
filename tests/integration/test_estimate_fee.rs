use std::sync::Arc;

use server::test_utils::assert_contains;
use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
use starknet_rs_accounts::{
    Account, AccountError, AccountFactory, AccountFactoryError, ConnectedAccount, ExecutionEncoder,
    ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::contract::legacy::LegacyContractClass;
use starknet_rs_core::types::{
    BlockId, BlockTag, BroadcastedDeclareTransactionV1, BroadcastedDeclareTransactionV3,
    BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV1, BroadcastedInvokeTransactionV3,
    BroadcastedTransaction, Call, DataAvailabilityMode, FeeEstimate, Felt, FunctionCall,
    ResourceBounds, ResourceBoundsMapping, StarknetError, TransactionExecutionErrorData,
};
use starknet_rs_core::utils::{
    cairo_short_string_to_felt, get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
};
use starknet_rs_providers::jsonrpc::{HttpTransport, JsonRpcError};
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
use starknet_rs_signers::{LocalWallet, Signer};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_CONTRACT_PATH, CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH,
    CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID, ETH_ERC20_CONTRACT_ADDRESS,
    QUERY_VERSION_OFFSET, UDC_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    assert_json_rpc_errors_equal, assert_tx_reverted, assert_tx_successful, extract_json_rpc_error,
    get_deployable_account_signer, get_flattened_sierra_contract_and_casm_hash,
};

fn assert_fee_estimation(fee_estimation: &FeeEstimate) {
    assert_eq!(
        fee_estimation.gas_price * fee_estimation.gas_consumed
            + fee_estimation.data_gas_consumed * fee_estimation.data_gas_price,
        fee_estimation.overall_fee
    );
    assert!(fee_estimation.overall_fee > Felt::ZERO, "Checking fee_estimation: {fee_estimation:?}");
}

fn multiply_field_element(field_element: Felt, multiplier: f64) -> Felt {
    let (_, parts) = field_element.to_bigint().to_u64_digits();
    assert_eq!(parts.len(), 1);

    ((parts[0] as f64 * multiplier) as u128).into()
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
    let deployment = account_factory.deploy_v1(salt);
    let deployment_address = deployment.address();
    let fee_estimation = account_factory
        .deploy_v1(salt)
        .fee_estimate_multiplier(1.0)
        .nonce(new_account_nonce)
        .estimate_fee()
        .await
        .unwrap();
    assert_fee_estimation(&fee_estimation);

    // fund the account before deployment
    let mint_amount = fee_estimation.overall_fee * Felt::TWO;
    devnet.mint(deployment_address, mint_amount.to_biguint().try_into().unwrap()).await;

    // try sending with insufficient max fee
    let unsuccessful_deployment_tx = account_factory
        .deploy_v1(salt)
        .max_fee(fee_estimation.overall_fee - Felt::ONE)
        .nonce(new_account_nonce)
        .send()
        .await;
    match unsuccessful_deployment_tx {
        Err(AccountFactoryError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientMaxFee,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    // try sending with sufficient max fee
    let successful_deployment = account_factory
        .deploy_v1(salt)
        .max_fee(multiply_field_element(fee_estimation.overall_fee, 1.1))
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
    let deployment = account_factory.deploy_v1(salt);
    match deployment.estimate_fee().await {
        Err(AccountFactoryError::Provider(provider_error)) => assert_json_rpc_errors_equal(
            extract_json_rpc_error(provider_error).unwrap(),
            JsonRpcError {
                code: 41,
                message: "Transaction execution error".into(),
                data: Some(serde_json::json!({
                    "transaction_index": 0,
                    "execution_error": {
                        "contract_address": deployment.address(),
                        "class_hash": invalid_class_hash,
                        "selector": null,
                        "error": format!("Class with hash {} is not declared.\n", invalid_class_hash.to_fixed_hex_string())
                    }
                })),
            },
        ),
        other => panic!("Unexpected response: {other:?}"),
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
        ExecutionEncoding::New,
    );

    let fee_estimation = account
        .declare_legacy(Arc::clone(&contract_artifact))
        .nonce(Felt::ZERO)
        .fee_estimate_multiplier(1.0)
        .estimate_fee()
        .await
        .unwrap();
    assert_fee_estimation(&fee_estimation);

    // try sending with insufficient max fee
    let unsuccessful_declare_tx = account
        .declare_legacy(Arc::clone(&contract_artifact))
        .nonce(Felt::ZERO)
        .max_fee(fee_estimation.overall_fee - Felt::ONE)
        .send()
        .await;
    match unsuccessful_declare_tx {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientMaxFee,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    // try sending with sufficient max fee
    let successful_declare_tx = account
        .declare_legacy(contract_artifact)
        .nonce(Felt::ZERO)
        .max_fee(multiply_field_element(fee_estimation.overall_fee, 1.1))
        .send()
        .await
        .unwrap();
    assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client).await;
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
        ExecutionEncoding::New,
    );

    // two times in a row should produce the same result
    let mut fee_estimations: Vec<FeeEstimate> = vec![];
    for _ in 0..2 {
        let fee_estimation = account
            .declare_v2(Arc::clone(&flattened_contract_artifact), casm_hash)
            .nonce(Felt::ZERO)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);
        fee_estimations.push(fee_estimation);
    }
    assert_eq!(fee_estimations[0], fee_estimations[1]);
    let fee_estimation = &fee_estimations[0];

    // try sending with insufficient max fee
    let unsuccessful_declare_tx = account
        .declare_v2(Arc::clone(&flattened_contract_artifact), casm_hash)
        .nonce(Felt::ZERO)
        .max_fee(fee_estimation.overall_fee - Felt::ONE)
        .send()
        .await;
    match unsuccessful_declare_tx {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientMaxFee,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    // try sending with sufficient max fee
    let successful_declare_tx = account
        .declare_v2(Arc::clone(&flattened_contract_artifact), casm_hash)
        .nonce(Felt::ZERO)
        .max_fee(multiply_field_element(fee_estimation.overall_fee, 1.1))
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
    let contract_json = dummy_cairo_0_contract_class();
    let contract_artifact: Arc<LegacyContractClass> =
        Arc::new(serde_json::from_value(contract_json.inner).unwrap());
    let class_hash = contract_artifact.class_hash().unwrap();

    // declare class
    let declaration_result = account
        .declare_legacy(contract_artifact.clone())
        .nonce(Felt::ZERO)
        .max_fee(Felt::from(1e18 as u128))
        .send()
        .await
        .unwrap();
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
        .deploy_v1(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    // prepare the call used in estimation and actual invoke
    let increase_amount = Felt::from(100u128);
    let invoke_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![increase_amount],
    }];

    // estimate the fee
    let fee_estimation = account
        .execute_v1(invoke_calls.clone())
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
    let insufficient_max_fee = fee_estimation.overall_fee - Felt::ONE;
    let unsuccessful_invoke_tx = account
        .execute_v1(invoke_calls.clone())
        .max_fee(insufficient_max_fee)
        .send()
        .await
        .unwrap();
    let balance_after_insufficient =
        devnet.json_rpc_client.call(call.clone(), BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(balance_after_insufficient, vec![Felt::ZERO]);

    assert_tx_reverted(
        &unsuccessful_invoke_tx.transaction_hash,
        &devnet.json_rpc_client,
        &["Insufficient max fee"],
    )
    .await;

    // invoke with sufficient max_fee
    let sufficient_max_fee = multiply_field_element(fee_estimation.overall_fee, 1.1);

    account.execute_v1(invoke_calls).max_fee(sufficient_max_fee).send().await.unwrap();
    let balance_after_sufficient =
        devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(balance_after_sufficient, vec![increase_amount]);
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
        account.declare_v2(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
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
        .deploy_v1(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    let panic_reason = "custom little reason";
    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("create_panic").unwrap(),
        calldata: vec![cairo_short_string_to_felt(panic_reason).unwrap()],
    }];

    let invoke_err = account
        .execute_v1(calls.clone())
        .nonce(account.get_nonce().await.unwrap())
        .max_fee(Felt::ZERO)
        .estimate_fee()
        .await
        .unwrap_err();

    match invoke_err {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                transaction_index,
                execution_error,
                ..
            }),
        )) => {
            assert_eq!(transaction_index, 0);
            assert_contains(&execution_error, panic_reason);
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
        account.declare_v2(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
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
        .deploy_v1(constructor_calldata, salt, false)
        .send()
        .await
        .expect("Cannot deploy");

    let expected_version = QUERY_VERSION_OFFSET + Felt::ONE;
    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("assert_version").unwrap(),
        calldata: vec![expected_version],
    }];

    match account.execute_v1(calls).estimate_fee().await {
        Ok(_) => (),
        other => panic!("Unexpected result: {other:?}"),
    }
}

async fn broadcasted_invoke_v1_for_estimation(
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    signer: &LocalWallet,
    to_address: Felt,
    selector: Felt,
    calldata: &[Felt],
    nonce: Felt,
) -> Result<BroadcastedTransaction, anyhow::Error> {
    let calls = vec![Call { to: to_address, selector, calldata: calldata.to_vec() }];
    let calldata = account.encode_calls(&calls);

    let max_fee = Felt::ZERO;
    let prepared_invoke = account.execute_v1(calls).nonce(nonce).max_fee(max_fee).prepared()?;

    let is_query = false;
    let signature = signer.sign_hash(&prepared_invoke.transaction_hash(is_query)).await?;

    Ok(BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(
        BroadcastedInvokeTransactionV1 {
            max_fee,
            signature: vec![signature.r, signature.s],
            nonce,
            sender_address: account.address(),
            calldata,
            is_query,
        },
    )))
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
    let contract_json = dummy_cairo_0_contract_class();
    let contract_class: Arc<LegacyContractClass> =
        Arc::new(serde_json::from_value(contract_json.inner).unwrap());

    let declaration_nonce = Felt::ZERO;
    let declaration_max_fee = Felt::ZERO;
    let class_hash = contract_class.class_hash().unwrap();
    let prepared_legacy_declaration = account
        .declare_legacy(contract_class.clone())
        .max_fee(declaration_max_fee)
        .nonce(declaration_nonce)
        .prepared()
        .unwrap();

    let query_only = false;
    let declaration_signature = signer
        .sign_hash(&prepared_legacy_declaration.transaction_hash(query_only).unwrap())
        .await
        .unwrap();

    devnet
        .json_rpc_client
        .estimate_fee(
            [
                BroadcastedTransaction::Declare(
                    starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                        BroadcastedDeclareTransactionV1 {
                            max_fee: declaration_max_fee,
                            signature: vec![declaration_signature.r, declaration_signature.s],
                            nonce: declaration_nonce,
                            sender_address: account_address,
                            contract_class: contract_class.compress().unwrap().into(),
                            is_query: query_only,
                        },
                    ),
                ),
                broadcasted_invoke_v1_for_estimation(
                    &account,
                    &signer,
                    UDC_CONTRACT_ADDRESS,
                    get_selector_from_name("deployContract").unwrap(),
                    &[
                        class_hash,
                        Felt::from_hex_unchecked("0x123"), // salt
                        Felt::ZERO,
                        Felt::ZERO,
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
                broadcasted_invoke_v1_for_estimation(
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
                broadcasted_invoke_v1_for_estimation(
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
                &execution_error,
                &format!(
                    "Entry point EntryPointSelector({}) not found in contract",
                    non_existent_selector.to_hex_string()
                ),
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
                BroadcastedTransaction::Declare(
                    starknet_rs_core::types::BroadcastedDeclareTransaction::V3(
                        BroadcastedDeclareTransactionV3 {
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
                        },
                    ),
                ),
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(
                    BroadcastedInvokeTransactionV3 {
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
                    },
                )),
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
            assert_contains(&execution_error, "invalid signature");
        }
        other => panic!("Unexpected error: {:?}", other),
    }
}
