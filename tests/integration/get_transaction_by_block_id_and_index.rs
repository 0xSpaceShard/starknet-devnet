use starknet_rs_core::types::{
    BlockId, BlockTag, Felt, InvokeTransaction, StarknetError, Transaction, TransactionResponseFlag,
};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::create_proof_bearing_transaction;

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
    let proof_bearing_tx = create_proof_bearing_transaction(&devnet).await;
    let submitted_proof_facts = proof_bearing_tx.submitted_proof_facts.clone();

    let tx_without_flags = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(
            BlockId::Hash(proof_bearing_tx.containing_block_hash),
            proof_bearing_tx.transaction_index,
            None,
        )
        .await
        .unwrap();

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let tx_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(
            BlockId::Hash(proof_bearing_tx.containing_block_hash),
            proof_bearing_tx.transaction_index,
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
                returned_proof_facts, submitted_proof_facts,
                "proof_facts returned by get_transaction_by_block_id_and_index should match sent \
                 proof_facts"
            );
        }
        _ => panic!("Expected invoke v3 transaction with IncludeProofFacts flag"),
    }
}

#[tokio::test]
async fn get_transaction_by_block_id_and_index_include_proof_facts_on_non_proof_bearing_tx_returns_empty_array()
 {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let _minting_hash = devnet.mint(Felt::ONE, 1).await;

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let tx_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_transaction_by_block_id_and_index(
            BlockId::Tag(BlockTag::Latest),
            0,
            Some(&response_flags),
        )
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
