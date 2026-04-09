use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{BlockId, BlockTag, Call, Felt, StarknetError, TransactionReceipt};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, STRK_ERC20_CONTRACT_ADDRESS};
use crate::common::utils::{assert_tx_succeeded_accepted, felt_to_u128};
use crate::messaging::{DUMMY_L1_ADDRESS, increase_balance, setup_devnet};

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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();
    let result = devnet.prove_transaction(invoke_for_prove).await;

    assert!(!result.proof_base64.is_empty(), "proof should be a non-empty base64 string");
    assert_eq!(result.proof_facts_hex.len(), 9, "proof_facts should have 9 elements");
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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, false).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;
    let proof = prove_result.proof_base64;
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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, false).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let wrong_proof =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, vec![0u8; 8]);
    let proof_facts = prove_result.proof_facts;
    let fees = account
        .execute_v3(tx_calls.clone())
        .nonce(tx_nonce)
        .tip(0)
        .proof(prove_result.proof_base64)
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

    match error {
        starknet_rs_accounts::AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InvalidProof,
        )) => (),
        _ => panic!("Invalid error: {error:?}"),
    }
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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, false).await.unwrap();
    let prove_result = devnet.prove_transaction(invoke_for_prove).await;

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let proof_only = prove_result.proof_base64;
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

    match error {
        starknet_rs_accounts::AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InvalidProof,
        )) => (),
        _ => panic!("Invalid error: {error:?}"),
    }
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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();
    let result1 = devnet.prove_transaction(invoke_for_prove.clone()).await;
    let result2 = devnet.prove_transaction(invoke_for_prove).await;

    assert_eq!(result1.proof_base64, result2.proof_base64, "Same tx should produce same proof");
    assert_eq!(
        result1.proof_facts_hex, result2.proof_facts_hex,
        "Same tx should produce same proof_facts"
    );
}

#[tokio::test]
async fn prove_transaction_differs_on_different_block_ids() {
    let devnet = BackgroundDevnet::spawn_forkable_devnet()
        .await
        .expect("Could not start Devnet with full state archive");

    // First, create at least 15 blocks to ensure we have sufficient chain history
    for _ in 0..15 {
        devnet.create_block().await.unwrap();
    }

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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();

    // Prove at block 10
    let block_id_10 = BlockId::Number(10u64);
    let result_at_block_10 =
        devnet.prove_transaction_at_block(invoke_for_prove.clone(), block_id_10).await;

    // Prove the same transaction at block 15 (no blocks created between)
    let block_id_15 = BlockId::Number(15u64);
    let result_at_block_15 = devnet.prove_transaction_at_block(invoke_for_prove, block_id_15).await;

    // Proofs should differ because they include different block numbers and hashes
    assert_ne!(
        result_at_block_10.proof_base64, result_at_block_15.proof_base64,
        "Proofs should differ for different block numbers (10 vs 15)"
    );
    assert_ne!(
        result_at_block_10.proof_facts_hex, result_at_block_15.proof_facts_hex,
        "Proof facts should differ for different block numbers"
    );
}

#[tokio::test]
async fn invoke_in_proof_mode_none_accepts_without_proof_or_with_wrong_proof() {
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
    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, false).await.unwrap();
    let prove_result = devnet_with_proofs.prove_transaction(invoke_for_prove).await;

    // Ensure devnet_none has enough blocks for the block-hash retention buffer
    for _ in 0..11 {
        devnet_none.create_block().await.unwrap();
    }

    // Re-fetch gas prices after block creation (they may have changed)
    let none_block = devnet_none
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();
    let tx_l1_gas_price = felt_to_u128(none_block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(none_block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(none_block.l2_gas_price().price_in_fri);

    let nonce_with_valid_proof = devnet_none
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), none_account_address)
        .await
        .unwrap();
    let fees_with_valid_proof = none_account
        .execute_v3(tx_calls.clone())
        .nonce(nonce_with_valid_proof)
        .tip(0)
        .proof(prove_result.proof_base64.clone())
        .proof_facts(prove_result.proof_facts.clone())
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
        .proof(prove_result.proof_base64.clone())
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
    let wrong_proof =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, vec![0u8; 8]);
    let fees_with_wrong_proof = none_account
        .execute_v3(tx_calls.clone())
        .nonce(nonce_with_wrong_proof)
        .tip(0)
        .proof(wrong_proof.clone())
        .proof_facts(prove_result.proof_facts.clone())
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
        .proof(wrong_proof)
        .proof_facts(prove_result.proof_facts)
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

#[tokio::test]
async fn invoke_in_proof_mode_none_rejects_wrong_proof_facts() {
    // In none mode, proof is ignored but proof_facts are preserved and validated.
    // Sending invalid proof_facts should cause a transaction execution error.
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

    // Need enough blocks for block-hash retention buffer
    for _ in 0..11 {
        devnet_none.create_block().await.unwrap();
    }

    let block = devnet_none
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let tx_calls = vec![transfer_call(Felt::ONE, Felt::from(1000u64))];
    let nonce = devnet_none
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), none_account_address)
        .await
        .unwrap();

    let wrong_proof_facts = vec![Felt::ONE; 9];
    let error = none_account
        .execute_v3(tx_calls)
        .l1_gas(5_000_000)
        .l1_data_gas(1_000_000)
        .l2_gas(2_500_000_000)
        .l1_gas_price(felt_to_u128(block.l1_gas_price().price_in_fri))
        .l1_data_gas_price(felt_to_u128(block.l1_data_gas_price().price_in_fri))
        .l2_gas_price(felt_to_u128(block.l2_gas_price().price_in_fri))
        .nonce(nonce)
        .proof_facts(wrong_proof_facts)
        .tip(0)
        .send()
        .await
        .unwrap_err();

    match error {
        starknet_rs_accounts::AccountError::Provider(ProviderError::StarknetError(
            StarknetError::ValidationFailure(msg),
        )) => {
            assert!(
                msg.contains("ProofFacts parse error"),
                "Expected ProofFacts parse error, got: {msg}"
            );
        }
        _ => panic!("Expected ValidationFailure for invalid proof_facts, got: {error:?}"),
    }
}

#[tokio::test]
async fn prove_transaction_returns_l2_to_l1_messages_for_withdraw() {
    let (devnet, account, contract_address) =
        setup_devnet(&["--account-class", "cairo1"]).await.unwrap();
    let account_address = account.address();

    // increase_balance for user before withdraw
    let user = Felt::ONE;
    let amount = Felt::ONE;
    increase_balance(account.clone(), contract_address, user, amount)
        .await
        .expect("increase_balance failed");

    // Build a withdraw transaction (produces L2→L1 message) for prove_transaction
    let withdraw_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("withdraw").unwrap(),
        calldata: vec![user, amount, DUMMY_L1_ADDRESS],
    }];

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
        .execute_v3(withdraw_calls.clone())
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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();
    let result = devnet.prove_transaction(invoke_for_prove).await;

    // Proof should still be valid
    assert!(!result.proof_base64.is_empty(), "proof should be non-empty");
    assert_eq!(result.proof_facts_hex.len(), 9, "proof_facts should have 9 elements");

    // l2_to_l1_messages should contain the withdraw message
    assert!(
        !result.l2_to_l1_messages.is_empty(),
        "l2_to_l1_messages should not be empty for a withdraw transaction"
    );

    let msg = &result.l2_to_l1_messages[0];
    assert_eq!(msg.from_address, contract_address, "message should be from the contract address");
    assert_eq!(msg.to_address, DUMMY_L1_ADDRESS, "message should be sent to the L1 address");
    assert!(!msg.payload.is_empty(), "message should have a payload");

    // Execute the same transaction and ensure emitted message matches prove_transaction output.
    let proof = result.proof_base64.clone();
    let proof_facts = result.proof_facts.clone();

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let latest_block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();
    let tx_l1_gas_price = felt_to_u128(latest_block.l1_gas_price().price_in_fri);
    let tx_l1_data_gas_price = felt_to_u128(latest_block.l1_data_gas_price().price_in_fri);
    let tx_l2_gas_price = felt_to_u128(latest_block.l2_gas_price().price_in_fri);

    let fees = account
        .execute_v3(withdraw_calls.clone())
        .nonce(nonce)
        .tip(0)
        .proof(proof.clone())
        .proof_facts(proof_facts.clone())
        .estimate_fee()
        .await
        .unwrap();

    let send_result = account
        .execute_v3(withdraw_calls)
        .l1_gas(fees.l1_gas_consumed)
        .l1_data_gas(fees.l1_data_gas_consumed)
        .l2_gas(fees.l2_gas_consumed)
        .l1_gas_price(tx_l1_gas_price)
        .l1_data_gas_price(tx_l1_data_gas_price)
        .l2_gas_price(tx_l2_gas_price)
        .nonce(nonce)
        .proof(proof)
        .proof_facts(proof_facts)
        .tip(0)
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&send_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let receipt = devnet
        .json_rpc_client
        .get_transaction_receipt(send_result.transaction_hash)
        .await
        .unwrap()
        .receipt;

    match receipt {
        TransactionReceipt::Invoke(invoke_receipt) => {
            assert_eq!(invoke_receipt.messages_sent.len(), result.l2_to_l1_messages.len());

            let executed_msg = &invoke_receipt.messages_sent[0];
            assert_eq!(executed_msg.from_address, msg.from_address);
            assert_eq!(executed_msg.to_address, msg.to_address);
            assert_eq!(executed_msg.payload, msg.payload);
        }
        other => panic!("Expected invoke receipt, got: {other:?}"),
    }
}

#[tokio::test]
async fn prove_transaction_returns_empty_messages_for_simple_transfer() {
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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();
    let result = devnet.prove_transaction(invoke_for_prove).await;

    assert!(!result.proof_base64.is_empty(), "proof should be non-empty");
    assert_eq!(result.proof_facts_hex.len(), 9, "proof_facts should have 9 elements");

    // A simple transfer produces no L2→L1 messages
    assert!(
        result.l2_to_l1_messages.is_empty(),
        "l2_to_l1_messages should be empty for a simple transfer, got: {:?}",
        result.l2_to_l1_messages
    );
}

#[tokio::test]
async fn prove_transaction_returns_error_on_execution_failure() {
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

    // Call a non-existent contract to trigger execution failure
    let calls = vec![Call {
        to: Felt::from_hex_unchecked("0xdeadbeef"),
        selector: get_selector_from_name("nonexistent_function").unwrap(),
        calldata: vec![],
    }];

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

    let invoke_for_prove = prepared.get_invoke_request(false, false).await.unwrap();

    let mut transaction =
        serde_json::to_value(invoke_for_prove).expect("Failed to serialize transaction");
    if let Some(obj) = transaction.as_object_mut() {
        obj.remove("proof");
        obj.remove("proof_facts");
    }

    let error = devnet
        .send_custom_rpc(
            "starknet_proveTransaction",
            serde_json::json!({
                "block_id": "latest",
                "transaction": transaction
            }),
        )
        .await
        .expect_err("prove_transaction should fail for non-executable transaction");

    assert!(
        error.message.contains("Transaction execution failed"),
        "Error should mention transaction execution failure, got: {error}"
    );
}
