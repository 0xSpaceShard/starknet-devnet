use std::sync::Arc;

use starknet_rs_accounts::{
    Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, Felt, InvokeTransaction, StarknetError, Transaction,
    TransactionResponseFlag,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, ETH_ERC20_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    assert_tx_succeeded_accepted, create_proof_bearing_transaction, get_deployable_account_signer,
    get_simple_contract_artifacts,
};

#[tokio::test]
async fn get_declare_v3_transaction_by_hash_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let (contract_class, casm_hash) = get_simple_contract_artifacts();

    let (signer, address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::Legacy,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let declare_result = account
        .declare_v3(Arc::new(contract_class), casm_hash)
        .nonce(Felt::ZERO)
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&declare_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_deploy_account_transaction_by_hash_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let signer = get_deployable_account_signer();

    let factory = OpenZeppelinAccountFactory::new(
        Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH),
        constants::CHAIN_ID,
        signer,
        devnet.clone_provider(),
    )
    .await
    .unwrap();

    let salt = Felt::from_hex_unchecked("0x123");
    let deployment = factory.deploy_v3(salt).gas_estimate_multiplier(1.0);
    let deployment_address = deployment.address();
    let fee_estimation = deployment.estimate_fee().await.unwrap();

    // fund the account before deployment
    let mint_amount = fee_estimation.overall_fee * 2;
    devnet.mint(deployment_address, mint_amount).await;

    let deploy_account_result = deployment.send().await.unwrap();
    assert_tx_succeeded_accepted(&deploy_account_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_invoke_v3_transaction_by_hash_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let invoke_tx_result = account
        .execute_v3(vec![Call {
            to: ETH_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,                 // recipient
                Felt::from(1_000_000_000), // low part of uint256
                Felt::ZERO,                // high part of uint256
            ],
        }])
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&invoke_tx_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_non_existing_transaction() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let result =
        devnet.json_rpc_client.get_transaction_by_hash(Felt::ZERO, None).await.unwrap_err();

    match result {
        ProviderError::StarknetError(StarknetError::TransactionHashNotFound) => (),
        _ => panic!("Invalid error: {result:?}"),
    }
}

#[tokio::test]
async fn get_transaction_by_hash_response_flags_control_proof_facts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let proof_bearing_tx = create_proof_bearing_transaction(&devnet).await;
    let submitted_proof_facts = proof_bearing_tx.submitted_proof_facts.clone();

    let tx_without_flags = devnet
        .json_rpc_client
        .get_transaction_by_hash(proof_bearing_tx.transaction_hash, None)
        .await
        .unwrap();

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let tx_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_transaction_by_hash(proof_bearing_tx.transaction_hash, Some(&response_flags))
        .await
        .unwrap();

    match tx_without_flags {
        Transaction::Invoke(InvokeTransaction::V3(tx)) => {
            assert!(
                tx.proof_facts.is_none(),
                "proof_facts should not be returned when response_flags are not provided"
            );
        }
        _ => panic!("Expected invoke v3 transaction without flags"),
    }

    match tx_with_proof_facts_flag {
        Transaction::Invoke(InvokeTransaction::V3(tx)) => {
            let returned_proof_facts = tx
                .proof_facts
                .expect("proof_facts should be returned when IncludeProofFacts is requested");
            assert_eq!(
                returned_proof_facts.len(),
                8,
                "proof_facts should contain expected 8 elements"
            );
            assert_eq!(
                returned_proof_facts, submitted_proof_facts,
                "proof_facts returned by get_transaction_by_hash should match sent proof_facts"
            );
        }
        _ => panic!("Expected invoke v3 transaction with IncludeProofFacts flag"),
    }
}

#[tokio::test]
async fn get_transaction_by_hash_include_proof_facts_on_non_proof_bearing_tx_returns_empty_array() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let minting_hash = devnet.mint(Felt::ONE, 1).await;

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let tx_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_transaction_by_hash(minting_hash, Some(&response_flags))
        .await
        .unwrap();

    match tx_with_proof_facts_flag {
        Transaction::Invoke(InvokeTransaction::V3(tx)) => {
            let returned_proof_facts = tx
                .proof_facts
                .expect("proof_facts should be present when IncludeProofFacts is requested");
            assert!(
                returned_proof_facts.is_empty(),
                "proof_facts should be an empty array for non-proof-bearing transactions"
            );
        }
        _ => panic!("Expected invoke v3 transaction with IncludeProofFacts flag"),
    }
}
