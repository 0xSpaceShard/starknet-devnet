use std::sync::Arc;

use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    Call, ContractExecutionError, Felt, StarknetError, TransactionExecutionErrorData,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::ProviderError;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, UDC_CONTRACT_ADDRESS, UDC_LEGACY_CONTRACT_ADDRESS,
    UDC_LEGACY_CONTRACT_CLASS_HASH,
};
use crate::common::utils::{
    assert_contains, assert_tx_succeeded_accepted, extract_message_error, extract_nested_error,
    get_simple_contract_artifacts,
};

// Testing of account deployment can be found in test_account_selection.rs

#[tokio::test]
async fn double_deployment_not_allowed() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    // declare
    let (contract_class, casm_hash) = get_simple_contract_artifacts();
    let declaration_result = account
        .declare_v3(Arc::new(contract_class), casm_hash)
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(5e7 as u64)
        .send()
        .await
        .unwrap();

    // prepare deployment
    let contract_factory = ContractFactory::new(declaration_result.class_hash, account.clone());
    let ctor_args = vec![Felt::ZERO]; // initial value
    let salt = Felt::from(10);
    let unique = false;

    // first deployment should be successful
    contract_factory.deploy_v3(ctor_args.clone(), salt, unique).send().await.unwrap();

    // second deployment should be unsuccessful
    match contract_factory.deploy_v3(ctor_args, salt, unique).send().await {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                transaction_index: 0,
                execution_error: ContractExecutionError::Nested(top_error),
            }),
        ))) => {
            assert_eq!(top_error.contract_address, account.address());
            assert_eq!(
                top_error.class_hash,
                Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)
            );
            assert_eq!(top_error.selector, get_selector_from_name("__execute__").unwrap());

            let udc_error = extract_nested_error(&top_error.error);
            assert_eq!(udc_error.contract_address, UDC_LEGACY_CONTRACT_ADDRESS);
            assert_eq!(udc_error.class_hash, UDC_LEGACY_CONTRACT_CLASS_HASH);
            assert_eq!(udc_error.selector, get_selector_from_name("deployContract").unwrap());

            let undeployed_contract_error = extract_nested_error(&udc_error.error);
            assert_eq!(undeployed_contract_error.class_hash, declaration_result.class_hash);
            assert_eq!(undeployed_contract_error.selector, Felt::ZERO); // constructor

            let msg_error = extract_message_error(&undeployed_contract_error.error);
            assert_contains(msg_error, "contract already deployed").unwrap();
        }
        other => panic!("Unexpected result: {other:?}"),
    };
}

#[tokio::test]
async fn cannot_deploy_undeclared_class() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    // skip declaration
    let (contract_class, _) = get_simple_contract_artifacts();

    // prepare deployment
    let contract_factory = ContractFactory::new(contract_class.class_hash(), account.clone());
    let ctor_args = vec![Felt::ZERO]; // initial value
    let salt = Felt::from(10);
    let unique = false;

    // deployment should fail
    match contract_factory.deploy_v3(ctor_args, salt, unique).send().await {
        Err(e) => assert_contains(&format!("{e:?}"), "not declared").unwrap(),
        other => panic!("Unexpected result: {other:?}"),
    };
}

#[tokio::test]
async fn test_all_udc_deployment_methods_supported() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    // declare
    let (contract_class, casm_hash) = get_simple_contract_artifacts();
    let declaration_result = account
        .declare_v3(Arc::new(contract_class), casm_hash)
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(5e7 as u64)
        .send()
        .await
        .unwrap();
    match assert_tx_succeeded_accepted(
        &declaration_result.transaction_hash,
        &devnet.json_rpc_client,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => panic!("Transaction failed: {}", e),
    };

    let mut salt = Felt::ONE;
    let legacy_deployment_method = "deployContract";
    for (udc_address, deployment_method) in [
        (UDC_LEGACY_CONTRACT_ADDRESS, legacy_deployment_method),
        (UDC_LEGACY_CONTRACT_ADDRESS, legacy_deployment_method),
        (UDC_CONTRACT_ADDRESS, "deploy_contract"),
    ] {
        let contract_invoke = vec![Call {
            to: udc_address,
            selector: get_selector_from_name(deployment_method).unwrap(),
            calldata: vec![
                declaration_result.class_hash, // the hash of the class whose instance is deployed
                salt,
                Felt::ONE, // unique or not_from_zero
                Felt::ONE, // ctor_args len
                Felt::ONE, // ctor_arg
            ],
        }];

        salt += Felt::ONE; // iI salt not changed, error: contract already deployed

        let invoke_result = account.execute_v3(contract_invoke.clone()).send().await.unwrap();
        match assert_tx_succeeded_accepted(&invoke_result.transaction_hash, &devnet.json_rpc_client)
            .await
        {
            Ok(_) => {}
            Err(e) => panic!("Transaction failed: {}", e),
        };
    }
}
