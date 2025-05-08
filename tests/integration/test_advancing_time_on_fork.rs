use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{
    Call, Felt, StarknetError, TransactionExecutionStatus, TransactionStatus,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::{Provider, ProviderError};
use starknet_rs_signers::{LocalWallet, SigningKey};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::{
    ImpersonationAction, assert_contains, declare_v3_deploy_v3, extract_message_error,
    extract_nested_error, get_timestamp_asserter, increase_time,
};

#[tokio::test]
async fn tx_resource_estimation_fails_on_forked_devnet_with_impersonation_unless_time_incremented()
{
    let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

    let (signer, address) = origin_devnet.get_first_predeployed_account().await;
    let origin_account = SingleOwnerAccount::new(
        &origin_devnet.json_rpc_client,
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) = get_timestamp_asserter();

    let lock_interval = 86_400;
    let ctor_args = &[Felt::from(lock_interval)];
    let (_, contract_address) =
        declare_v3_deploy_v3(&origin_account, contract_class, casm_hash, ctor_args).await.unwrap();

    // Spawn a forked Devnet; use random seed to force predeployment of new accounts
    let fork_args = ["--fork-network", &origin_devnet.url, "--seed", "18726"];
    let forked_devnet = BackgroundDevnet::spawn_with_additional_args(&fork_args).await.unwrap();

    // Create a new, dummy account, which should work after activating impersonation
    let fork_account = SingleOwnerAccount::new(
        &forked_devnet.json_rpc_client,
        LocalWallet::from(SigningKey::from_secret_scalar(Felt::TWO)),
        Felt::THREE, // dummy address
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    forked_devnet
        .execute_impersonation_action(&ImpersonationAction::AutoImpersonate)
        .await
        .unwrap();

    let time_check_selector = get_selector_from_name("check_time").unwrap();
    let time_check_call =
        Call { to: contract_address, selector: time_check_selector, calldata: vec![] };

    // A failure is expected without time change.
    let error =
        fork_account.execute_v3(vec![time_check_call.clone()]).estimate_fee().await.unwrap_err();
    match error {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(error_data),
        )) => {
            assert_eq!(error_data.transaction_index, 0);

            let root_error = extract_nested_error(&error_data.execution_error);
            assert_eq!(root_error.contract_address, fork_account.address());
            assert_eq!(root_error.selector, get_selector_from_name("__execute__").unwrap());

            // Currently the root error is twice mentioned, so we extract twice
            let inner_error = extract_nested_error(&root_error.error);
            let inner_error = extract_nested_error(&inner_error.error);
            assert_eq!(inner_error.contract_address, contract_address);
            assert_eq!(inner_error.selector, time_check_selector);

            let message = extract_message_error(&inner_error.error);
            assert_contains(message, "Wait a bit more");
        }
        other => panic!("Invalid error: {other:?}"),
    }

    // Increasing the system timestamp should make the estimation succeed
    increase_time(&forked_devnet, lock_interval).await;
    fork_account.execute_v3(vec![time_check_call]).estimate_fee().await.unwrap();
}

#[tokio::test]
async fn tx_execution_fails_on_forked_devnet_with_impersonation_unless_time_incremented() {
    let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

    let (signer, address) = origin_devnet.get_first_predeployed_account().await;
    let origin_account = SingleOwnerAccount::new(
        &origin_devnet.json_rpc_client,
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) = get_timestamp_asserter();

    let lock_interval = 86_400;
    let ctor_args = &[Felt::from(lock_interval)];
    let (_, contract_address) =
        declare_v3_deploy_v3(&origin_account, contract_class, casm_hash, ctor_args).await.unwrap();

    // Spawn a forked Devnet; use random seed to force predeployment of new accounts
    let fork_args = ["--fork-network", &origin_devnet.url, "--seed", "18726"];
    let forked_devnet = BackgroundDevnet::spawn_with_additional_args(&fork_args).await.unwrap();

    // Create a new, dummy account, which should work after activating impersonation
    let fork_account = SingleOwnerAccount::new(
        &forked_devnet.json_rpc_client,
        LocalWallet::from(SigningKey::from_secret_scalar(Felt::TWO)),
        Felt::THREE, // dummy address
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    forked_devnet
        .execute_impersonation_action(&ImpersonationAction::AutoImpersonate)
        .await
        .unwrap();

    let time_check_selector = get_selector_from_name("check_time").unwrap();
    let time_check_call =
        Call { to: contract_address, selector: time_check_selector, calldata: vec![] };

    // A failure is expected without time change.
    let reverted_tx = fork_account
        .execute_v3(vec![time_check_call.clone()])
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(1e7 as u64)
        .send()
        .await
        .unwrap();

    match forked_devnet.json_rpc_client.get_transaction_status(reverted_tx.transaction_hash).await {
        Ok(TransactionStatus::AcceptedOnL2(tx_details)) => {
            assert_eq!(tx_details.status(), TransactionExecutionStatus::Reverted);
            assert_contains(tx_details.revert_reason().unwrap(), "Wait a bit more");
        }
        other => panic!("Unexpected tx: {other:?}"),
    }

    // Increasing the system timestamp should make the tx succeed (and the implicit fee estimation)
    increase_time(&forked_devnet, lock_interval).await;
    fork_account.execute_v3(vec![time_check_call]).send().await.unwrap();
}
