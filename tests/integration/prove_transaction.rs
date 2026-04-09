use std::sync::Arc;

use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{BlockId, BlockTag, Call, Felt, StarknetError};
use starknet_rs_core::utils::{UdcUniqueness, get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
use starknet_rs_signers::LocalWallet;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, CHAIN_ID, STRK_ERC20_CONTRACT_ADDRESS};
use crate::common::utils::{
    assert_tx_succeeded_accepted, felt_to_u128, get_messaging_contract_artifacts,
    new_contract_factory,
};

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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
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

    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
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
    let invoke_for_prove = prepared_for_prove.get_invoke_request(false, true).await.unwrap();
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

/// Deploy the L1-L2 messaging contract and return its address.
async fn deploy_l2_msg_contract(
    account: &Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
) -> Felt {
    let (sierra_class, casm_class_hash) = get_messaging_contract_artifacts();
    let sierra_class_hash = sierra_class.class_hash();
    account
        .declare_v3(Arc::new(sierra_class), casm_class_hash)
        .send()
        .await
        .expect("Failed to declare messaging contract");

    let contract_factory = new_contract_factory(sierra_class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![];
    let contract_address = get_udc_deployed_address(
        salt,
        sierra_class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v3(constructor_calldata, salt, false)
        .nonce(Felt::ONE)
        .send()
        .await
        .expect("Failed to deploy messaging contract");

    contract_address
}

#[tokio::test]
async fn prove_transaction_returns_l2_to_l1_messages_for_withdraw() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
        .await
        .expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    let contract_address = deploy_l2_msg_contract(&account).await;

    // increase_balance for user before withdraw
    let user = Felt::ONE;
    let amount = Felt::ONE;
    let increase_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![user, amount],
    }];
    account
        .execute_v3(increase_calls)
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(5e7 as u64)
        .send()
        .await
        .expect("increase_balance failed");

    // Build a withdraw transaction (produces L2→L1 message) for prove_transaction
    let dummy_l1_address = Felt::from_hex_unchecked("0xc662c410c0ecf747543f5ba90660f6abebd9c8c4");
    let withdraw_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("withdraw").unwrap(),
        calldata: vec![user, amount, dummy_l1_address],
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
        .execute_v3(withdraw_calls)
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

    // Proof should still be valid
    assert!(!result.proof_base64.is_empty(), "proof should be non-empty");
    assert_eq!(result.proof_facts_hex.len(), 9, "proof_facts should have 9 elements");

    // l2_to_l1_messages should contain the withdraw message
    assert!(
        !result.l2_to_l1_messages.is_empty(),
        "l2_to_l1_messages should not be empty for a withdraw transaction"
    );

    let msg = &result.l2_to_l1_messages[0];
    assert_eq!(msg.from_address, account.address(), "message should be from the sender account");
    assert_eq!(msg.to_address, dummy_l1_address, "message should be sent to the L1 address");
    assert!(!msg.payload.is_empty(), "message should have a payload");
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

    let invoke_for_prove = prepared.get_invoke_request(false, true).await.unwrap();
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

    let invoke_for_prove = prepared.get_invoke_request(false, true).await.unwrap();

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
