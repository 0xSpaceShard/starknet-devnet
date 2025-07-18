use std::sync::Arc;

use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{Call, Felt, InvokeTransactionResult, StarknetError};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::ProviderError;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH, CHAIN_ID, INVALID_ACCOUNT_SIERRA_PATH,
};
use crate::common::utils::{
    assert_contains, declare_v3_deploy_v3, get_flattened_sierra_contract_and_casm_hash,
    get_simple_contract_artifacts,
};

#[tokio::test]
async fn test_failed_validation_with_expected_message() {
    let args = ["--account-class-custom", INVALID_ACCOUNT_SIERRA_PATH];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

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
    let (contract_artifact, casm_hash) = get_simple_contract_artifacts();
    let contract_artifact = Arc::new(contract_artifact);

    // declare class
    let declaration_result = account
        .declare_v3(contract_artifact, casm_hash)
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(5e7 as u64)
        .send()
        .await;

    match declaration_result {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::ValidationFailure(message),
        ))) => assert_contains(&message, "FAILED VALIDATE DECLARE"),
        other => panic!("Unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn test_declaration_rejected_if_casm_hash_not_matching() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    let (contract_class, _) = get_simple_contract_artifacts();
    let dummy_casm_hash = Felt::ONE;

    let declaration_result = account
        .declare_v3(Arc::new(contract_class), dummy_casm_hash)
        .nonce(Felt::ZERO)
        .send()
        .await;

    match declaration_result {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::CompiledClassHashMismatch,
        ))) => (),
        other => panic!("Unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn test_tx_status_content_of_failed_invoke() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (sierra, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (_, contract_address) =
        declare_v3_deploy_v3(&account, sierra, casm_hash, &[]).await.unwrap();

    let InvokeTransactionResult { transaction_hash } = account
        .execute_v3(vec![Call {
            to: contract_address,
            selector: get_selector_from_name("create_panic").unwrap(),
            calldata: vec![],
        }])
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(5e7 as u64)
        .send()
        .await
        .unwrap();

    // Sending a custom request because starknet-rs was not adapted at the time of writing.
    let tx_status = devnet
        .send_custom_rpc(
            "starknet_getTransactionStatus",
            serde_json::json!({ "transaction_hash": transaction_hash }),
        )
        .await
        .unwrap();

    assert_eq!(tx_status["finality_status"], "ACCEPTED_ON_L2");
    assert_contains(tx_status["failure_reason"].as_str().unwrap(), "Error in the called contract");
    assert_eq!(tx_status["execution_status"], "REVERTED");
}
