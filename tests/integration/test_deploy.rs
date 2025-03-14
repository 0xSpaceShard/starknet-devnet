use std::sync::Arc;

use server::test_utils::assert_contains;
use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::Felt;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::get_simple_contract_in_sierra_and_compiled_class_hash;

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
        .gas(1e7 as u64)
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
        Err(e) => assert_contains(&format!("{e:?}"), "unavailable for deployment"),
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

    // second deployment should be unsuccessful
    match contract_factory.deploy_v3(ctor_args, salt, unique).send().await {
        Err(e) => assert_contains(&format!("{e:?}"), "not declared"),
        other => panic!("Unexpected result: {other:?}"),
    };
}
