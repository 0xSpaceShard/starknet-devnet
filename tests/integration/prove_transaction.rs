use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{BlockId, BlockTag, Call, Felt};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::Provider;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, STRK_ERC20_CONTRACT_ADDRESS};
use crate::common::utils::{assert_tx_succeeded_accepted, felt_to_u128};

/// Helper: build a simple transfer call
fn transfer_call(recipient: Felt, amount: Felt) -> Call {
    Call {
        to: STRK_ERC20_CONTRACT_ADDRESS,
        selector: get_selector_from_name("transfer").unwrap(),
        calldata: vec![recipient, amount, Felt::ZERO],
    }
}

#[tokio::test]
async fn prove_transaction_endpoint_returns_proof_and_proof_facts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let prepared = account
        .execute_v3(calls)
        .l1_gas(5_000_000)
        .l1_data_gas(1_000_000)
        .l2_gas(2_500_000_000)
        .l1_gas_price(felt_to_u128(block.l1_gas_price().price_in_fri))
        .l1_data_gas_price(felt_to_u128(block.l1_data_gas_price().price_in_fri))
        .l2_gas_price(felt_to_u128(block.l2_gas_price().price_in_fri))
        .nonce(nonce)
        .tip(0)
        .prepared()
        .unwrap();

    let invoke_for_prove = prepared.get_invoke_request(false, true).await.unwrap();
    let result = devnet.prove_transaction(invoke_for_prove).await;

    assert!(!result.proof_base64.is_empty(), "proof should be a non-empty base64 string");
    assert_eq!(result.proof_facts_hex.len(), 8, "proof_facts should have 8 elements");
}

#[tokio::test]
async fn invoke_with_valid_proof_is_accepted() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let tx_calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let tx_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let tx_l1_gas = 5_000_000;
    let tx_l1_data_gas = 1_000_000;
    let tx_l2_gas = 2_500_000_000;

    let prepared_for_prove = account
        .execute_v3(tx_calls.clone())
        .l1_gas(tx_l1_gas)
        .l1_data_gas(tx_l1_data_gas)
        .l2_gas(tx_l2_gas)
        .l1_gas_price(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .nonce(tx_nonce)
        .tip(0)
        .prepared()
        .unwrap();

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;
    let proof = prove_result.proof;
    let proof_facts = prove_result.proof_facts;

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let fees = account
        .execute_v3(tx_calls.clone())
        .nonce(tx_nonce)
        .tip(0)
        .proof(proof.clone())
        .proof_facts(proof_facts.clone())
        .estimate_fee()
        .await
        .unwrap();
    let tx_l1_gas_price = felt_to_u128(block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(block.l2_gas_price().price_in_fri);

    let send_result = account
        .execute_v3(tx_calls)
        .l1_gas(fees.l1_gas_consumed)
        .l1_data_gas(fees.l1_data_gas_consumed)
        .l2_gas(fees.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(tx_nonce)
        .proof(proof)
        .proof_facts(proof_facts)
        .tip(0)
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&send_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn invoke_with_wrong_proof_is_rejected() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let tx_calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let tx_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let tx_l1_gas = 5_000_000;
    let tx_l1_data_gas = 1_000_000;
    let tx_l2_gas = 2_500_000_000;
    let tx_l1_gas_price = felt_to_u128(block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(block.l2_gas_price().price_in_fri);

    let prepared_for_prove = account
        .execute_v3(tx_calls.clone())
        .l1_gas(tx_l1_gas)
        .l1_data_gas(tx_l1_data_gas)
        .l2_gas(tx_l2_gas)
        .l1_gas_price(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .nonce(tx_nonce)
        .tip(0)
        .prepared()
        .unwrap();

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let wrong_proof = vec![0u64; 8];
    let proof_facts = prove_result.proof_facts;
    let fees = account
        .execute_v3(tx_calls.clone())
        .nonce(tx_nonce)
        .tip(0)
        .proof(prove_result.proof)
        .proof_facts(proof_facts.clone())
        .estimate_fee()
        .await
        .unwrap();

    let error = account
        .execute_v3(tx_calls)
        .l1_gas(fees.l1_gas_consumed)
        .l1_data_gas(fees.l1_data_gas_consumed)
        .l2_gas(fees.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(tx_nonce)
        .proof(wrong_proof)
        .proof_facts(proof_facts)
        .tip(0)
        .send()
        .await
        .unwrap_err();

    let error_message = error.to_string();

    assert!(
        error_message.contains("Invalid proof")
            || error_message.contains("Account validation failed")
            || error_message.contains("Transaction execution error"),
        "Expected proof rejection, got: {}",
        error_message
    );
}

#[tokio::test]
async fn invoke_without_proof_in_devnet_mode_is_accepted() {
    // In devnet proof mode (default), transactions without proof fields should still be accepted
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let result = account.execute_v3(calls).send().await.unwrap();

    assert_tx_succeeded_accepted(&result.transaction_hash, &devnet.json_rpc_client).await.unwrap();
}

#[tokio::test]
async fn invoke_with_proof_only_and_no_proof_facts_is_rejected() {
    // Sending proof without proof_facts should fail
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let tx_calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let tx_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let tx_l1_gas = 5_000_000;
    let tx_l1_data_gas = 1_000_000;
    let tx_l2_gas = 2_500_000_000;
    let tx_l1_gas_price = felt_to_u128(block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(block.l2_gas_price().price_in_fri);

    let prepared_for_prove = account
        .execute_v3(tx_calls.clone())
        .l1_gas(tx_l1_gas)
        .l1_data_gas(tx_l1_data_gas)
        .l2_gas(tx_l2_gas)
        .l1_gas_price(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .nonce(tx_nonce)
        .tip(0)
        .prepared()
        .unwrap();

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let proof_only = prove_result.proof;
    let fees = account
        .execute_v3(tx_calls.clone())
        .nonce(tx_nonce)
        .tip(0)
        .proof(proof_only.clone())
        .proof_facts(prove_result.proof_facts)
        .estimate_fee()
        .await
        .unwrap();

    let error = account
        .execute_v3(tx_calls)
        .l1_gas(fees.l1_gas_consumed)
        .l1_data_gas(fees.l1_data_gas_consumed)
        .l2_gas(fees.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(tx_nonce)
        .proof(proof_only)
        .tip(0)
        .send()
        .await
        .unwrap_err();

    let error_message = error.to_string();

    println!("{}", error_message);

    assert!(
        error_message.contains("Invalid proof")
            || error_message.contains("Account validation failed")
            || error_message.contains("proof_facts")
            || error_message.contains("Transaction execution error"),
        "Expected proof rejection, got: {}",
        error_message
    );
}

#[tokio::test]
async fn prove_transaction_is_deterministic() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let prepared = account
        .execute_v3(calls)
        .l1_gas(5_000_000)
        .l1_data_gas(1_000_000)
        .l2_gas(2_500_000_000)
        .l1_gas_price(felt_to_u128(block.l1_gas_price().price_in_fri))
        .l1_data_gas_price(felt_to_u128(block.l1_data_gas_price().price_in_fri))
        .l2_gas_price(felt_to_u128(block.l2_gas_price().price_in_fri))
        .nonce(nonce)
        .tip(0)
        .prepared()
        .unwrap();

    let invoke_for_prove = prepared.get_invoke_request(false, true).await.unwrap();
    let result1 = devnet.prove_transaction(invoke_for_prove.clone()).await;
    let result2 = devnet.prove_transaction(invoke_for_prove).await;

    assert_eq!(result1.proof_base64, result2.proof_base64, "Same tx should produce same proof");
    assert_eq!(
        result1.proof_facts_hex, result2.proof_facts_hex,
        "Same tx should produce same proof_facts"
    );
}

#[tokio::test]
async fn invoke_in_proof_mode_none_accepts_with_or_without_any_proofs() {
    let devnet_none = BackgroundDevnet::spawn_with_additional_args(&["--proof-mode", "none"])
        .await
        .expect("Could not start Devnet in proof-mode none");

    let (none_signer, none_account_address) = devnet_none.get_first_predeployed_account().await;
    let mut none_account = SingleOwnerAccount::new(
        &devnet_none.json_rpc_client,
        none_signer,
        none_account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    none_account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let tx_calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let none_block = devnet_none
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let tx_l1_gas = 5_000_000;
    let tx_l1_data_gas = 1_000_000;
    let tx_l2_gas = 2_500_000_000;
    let tx_l1_gas_price = felt_to_u128(none_block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(none_block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(none_block.l2_gas_price().price_in_fri);

    let nonce_without_proof = devnet_none
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), none_account_address)
        .await
        .unwrap();
    let fees_without_proof = none_account
        .execute_v3(tx_calls.clone())
        .nonce(nonce_without_proof)
        .tip(0)
        .estimate_fee()
        .await
        .unwrap();
    let result_without_proof = none_account
        .execute_v3(tx_calls.clone())
        .l1_gas(fees_without_proof.l1_gas_consumed)
        .l1_data_gas(fees_without_proof.l1_data_gas_consumed)
        .l2_gas(fees_without_proof.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(nonce_without_proof)
        .tip(0)
        .send()
        .await
        .unwrap();
    assert_tx_succeeded_accepted(
        &result_without_proof.transaction_hash,
        &devnet_none.json_rpc_client,
    )
    .await
    .unwrap();

    let devnet_with_proofs = BackgroundDevnet::spawn().await.expect("Could not start proof devnet");
    let (proof_signer, proof_account_address) =
        devnet_with_proofs.get_first_predeployed_account().await;
    let mut proof_account = SingleOwnerAccount::new(
        &devnet_with_proofs.json_rpc_client,
        proof_signer,
        proof_account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    proof_account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let proof_nonce = devnet_with_proofs
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), proof_account_address)
        .await
        .unwrap();
    let prepared_for_prove = proof_account
        .execute_v3(tx_calls.clone())
        .l1_gas(tx_l1_gas)
        .l1_data_gas(tx_l1_data_gas)
        .l2_gas(tx_l2_gas)
        .l1_gas_price(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .nonce(proof_nonce)
        .tip(0)
        .prepared()
        .unwrap();
    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
    let prove_result = devnet_with_proofs.prove_transaction(invoke_for_prove).await;

    let nonce_with_valid_proof = devnet_none
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), none_account_address)
        .await
        .unwrap();
    let fees_with_valid_proof = none_account
        .execute_v3(tx_calls.clone())
        .nonce(nonce_with_valid_proof)
        .tip(0)
        .estimate_fee()
        .await
        .unwrap();
    let result_with_valid_proof = none_account
        .execute_v3(tx_calls.clone())
        .l1_gas(fees_with_valid_proof.l1_gas_consumed)
        .l1_data_gas(fees_with_valid_proof.l1_data_gas_consumed)
        .l2_gas(fees_with_valid_proof.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(nonce_with_valid_proof)
        .proof(prove_result.proof.clone())
        .proof_facts(prove_result.proof_facts.clone())
        .tip(0)
        .send()
        .await
        .unwrap();
    assert_tx_succeeded_accepted(
        &result_with_valid_proof.transaction_hash,
        &devnet_none.json_rpc_client,
    )
    .await
    .unwrap();

    let nonce_with_wrong_proof = devnet_none
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), none_account_address)
        .await
        .unwrap();
    let fees_with_wrong_proof = none_account
        .execute_v3(tx_calls.clone())
        .nonce(nonce_with_wrong_proof)
        .tip(0)
        .estimate_fee()
        .await
        .unwrap();
    let result_with_wrong_proof = none_account
        .execute_v3(tx_calls)
        .l1_gas(fees_with_wrong_proof.l1_gas_consumed)
        .l1_data_gas(fees_with_wrong_proof.l1_data_gas_consumed)
        .l2_gas(fees_with_wrong_proof.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(nonce_with_wrong_proof)
        .proof(vec![0u64; 8])
        .proof_facts(vec![Felt::ONE; 8])
        .tip(0)
        .send()
        .await
        .unwrap();
    assert_tx_succeeded_accepted(
        &result_with_wrong_proof.transaction_hash,
        &devnet_none.json_rpc_client,
    )
    .await
    .unwrap();
}
