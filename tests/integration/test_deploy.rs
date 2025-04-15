use std::sync::Arc;

use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    ContractExecutionError, Felt, StarknetError, TransactionExecutionErrorData,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::ProviderError;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use crate::common::utils::{
    assert_contains, extract_message_error, extract_nested_error,
    get_simple_contract_in_sierra_and_compiled_class_hash,
};

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
    let (contract_class, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();
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
            assert_eq!(udc_error.contract_address, UDC_CONTRACT_ADDRESS);
            assert_eq!(udc_error.class_hash, UDC_CONTRACT_CLASS_HASH);
            assert_eq!(udc_error.selector, get_selector_from_name("deployContract").unwrap());

            let undeployed_contract_error = extract_nested_error(&udc_error.error);
            assert_eq!(undeployed_contract_error.class_hash, declaration_result.class_hash);
            assert_eq!(undeployed_contract_error.selector, Felt::ZERO); // constructor

            let msg_error = extract_message_error(&undeployed_contract_error.error);
            assert_contains(msg_error, "contract already deployed");
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
    let (contract_class, _) = get_simple_contract_in_sierra_and_compiled_class_hash();

    // prepare deployment
    let contract_factory = ContractFactory::new(contract_class.class_hash(), account.clone());
    let ctor_args = vec![Felt::ZERO]; // initial value
    let salt = Felt::from(10);
    let unique = false;

    // deployment should fail
    match contract_factory.deploy_v3(ctor_args, salt, unique).send().await {
        Err(e) => assert_contains(&format!("{e:?}"), "not declared"),
        other => panic!("Unexpected result: {other:?}"),
    };
}
