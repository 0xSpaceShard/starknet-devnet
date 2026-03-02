use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, Felt, InvokeTransaction, StarknetError, Transaction,
    TransactionResponseFlag,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, ETH_ERC20_CONTRACT_ADDRESS};
use crate::common::utils::{assert_tx_succeeded_accepted, felt_to_u128};

#[tokio::test]
async fn get_transaction_by_block_id_and_index_happy_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let minting_hash = devnet.mint(Felt::ONE, 1).await;

    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 0, None)
        .await
        .unwrap();

    if let Transaction::Invoke(InvokeTransaction::V3(tx)) = result {
        assert_eq!(tx.transaction_hash, minting_hash);
    } else {
        panic!("Could not unpack the transaction from {result:?}");
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_wrong_index() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    devnet.mint(Felt::ONE, 1).await;

    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 1, None)
        .await
        .unwrap_err();

    match result {
        ProviderError::StarknetError(StarknetError::InvalidTransactionIndex) => (),
        _ => panic!("Invalid error: {result:?}"),
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_wrong_block() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let result = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(BlockId::Number(1), 1, None)
        .await
        .unwrap_err();

    match result {
        ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
        _ => panic!("Invalid error: {result:?}"),
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_response_flags_control_proof_facts() {
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

    let tx_calls = vec![Call {
        to: ETH_ERC20_CONTRACT_ADDRESS,
        selector: get_selector_from_name("transfer").unwrap(),
        calldata: vec![Felt::ONE, Felt::from(1_000_000_000u64), Felt::ZERO],
    }];

    let tx_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
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
    let sent_proof_facts = prove_result.proof_facts;
    assert_eq!(sent_proof_facts.len(), 8, "proof-bearing transaction should include proof_facts");

    for _ in 0..11 {
        devnet.create_block().await.unwrap();
    }

    let fees = account
        .execute_v3(tx_calls.clone())
        .nonce(tx_nonce)
        .tip(0)
        .proof(proof.clone())
        .proof_facts(sent_proof_facts.clone())
        .estimate_fee()
        .await
        .unwrap();

    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let send_result = account
        .execute_v3(tx_calls)
        .l1_gas(fees.l1_gas_consumed)
        .l1_data_gas(fees.l1_data_gas_consumed)
        .l2_gas(fees.l2_gas_consumed)
        .l1_gas_price(felt_to_u128(block.l1_gas_price().price_in_fri))
        .l1_data_gas_price(felt_to_u128(block.l1_data_gas_price().price_in_fri))
        .l2_gas_price(felt_to_u128(block.l2_gas_price().price_in_fri))
        .nonce(tx_nonce)
        .proof(proof)
        .proof_facts(sent_proof_facts.clone())
        .tip(0)
        .send()
        .await
        .unwrap();

    assert_tx_succeeded_accepted(&send_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == send_result.transaction_hash)
        .expect("sent transaction should exist in latest block") as u64;

    let tx_without_flags = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(
            BlockId::Hash(latest_block.block_hash),
            tx_index,
            None,
        )
        .await
        .unwrap();

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let tx_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(
            BlockId::Hash(latest_block.block_hash),
            tx_index,
            Some(&response_flags),
        )
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
                returned_proof_facts, sent_proof_facts,
                "proof_facts returned by get_transaction_by_block_id_and_index should match sent \
                 proof_facts"
            );
        }
        _ => panic!("Expected invoke v3 transaction with IncludeProofFacts flag"),
    }
}
