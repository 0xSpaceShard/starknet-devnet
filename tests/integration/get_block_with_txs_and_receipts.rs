use starknet_rs_core::types::{
    BlockId, InvokeTransaction, InvokeTransactionContent, MaybePreConfirmedBlockWithReceipts,
    MaybePreConfirmedBlockWithTxs, Transaction, TransactionContent, TransactionResponseFlag,
};
use starknet_rs_providers::Provider;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::utils::create_proof_bearing_transaction;

#[tokio::test]
async fn get_block_with_txs_response_flags_control_proof_facts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let proof_bearing_tx = create_proof_bearing_transaction(&devnet).await;
    let submitted_proof_facts = proof_bearing_tx.submitted_proof_facts.clone();

    let block_without_flags = devnet
        .json_rpc_client
        .get_block_with_txs(BlockId::Hash(proof_bearing_tx.containing_block_hash), None)
        .await
        .unwrap();

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let block_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_block_with_txs(
            BlockId::Hash(proof_bearing_tx.containing_block_hash),
            Some(&response_flags),
        )
        .await
        .unwrap();

    let tx_without_flags = match block_without_flags {
        MaybePreConfirmedBlockWithTxs::Block(block) => block
            .transactions
            .into_iter()
            .nth(proof_bearing_tx.transaction_index as usize)
            .expect("expected transaction at index in block"),
        _ => panic!("Expected confirmed block for block hash query"),
    };

    let tx_with_proof_facts = match block_with_proof_facts_flag {
        MaybePreConfirmedBlockWithTxs::Block(block) => block
            .transactions
            .into_iter()
            .nth(proof_bearing_tx.transaction_index as usize)
            .expect("expected transaction at index in block"),
        _ => panic!("Expected confirmed block for block hash query"),
    };

    match tx_without_flags {
        Transaction::Invoke(InvokeTransaction::V3(tx)) => {
            assert!(
                tx.proof_facts.is_none(),
                "proof_facts should not be returned when response_flags are not provided"
            );
        }
        _ => panic!("Expected invoke v3 transaction without flags"),
    }

    match tx_with_proof_facts {
        Transaction::Invoke(InvokeTransaction::V3(tx)) => {
            let returned_proof_facts = tx
                .proof_facts
                .expect("proof_facts should be returned when IncludeProofFacts is requested");
            assert_eq!(
                returned_proof_facts, submitted_proof_facts,
                "proof_facts returned by get_block_with_txs should match sent proof_facts"
            );
        }
        _ => panic!("Expected invoke v3 transaction with IncludeProofFacts flag"),
    }
}

#[tokio::test]
async fn get_block_with_receipts_response_flags_control_proof_facts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let proof_bearing_tx = create_proof_bearing_transaction(&devnet).await;
    let submitted_proof_facts = proof_bearing_tx.submitted_proof_facts.clone();

    let block_without_flags = devnet
        .json_rpc_client
        .get_block_with_receipts(BlockId::Hash(proof_bearing_tx.containing_block_hash), None)
        .await
        .unwrap();

    let response_flags = [TransactionResponseFlag::IncludeProofFacts];
    let block_with_proof_facts_flag = devnet
        .json_rpc_client
        .get_block_with_receipts(
            BlockId::Hash(proof_bearing_tx.containing_block_hash),
            Some(&response_flags),
        )
        .await
        .unwrap();

    let tx_without_flags = match block_without_flags {
        MaybePreConfirmedBlockWithReceipts::Block(block) => {
            block
                .transactions
                .into_iter()
                .nth(proof_bearing_tx.transaction_index as usize)
                .expect("expected transaction at index in block")
                .transaction
        }
        _ => panic!("Expected confirmed block for block hash query"),
    };

    let tx_with_proof_facts = match block_with_proof_facts_flag {
        MaybePreConfirmedBlockWithReceipts::Block(block) => {
            block
                .transactions
                .into_iter()
                .nth(proof_bearing_tx.transaction_index as usize)
                .expect("expected transaction at index in block")
                .transaction
        }
        _ => panic!("Expected confirmed block for block hash query"),
    };

    match tx_without_flags {
        TransactionContent::Invoke(InvokeTransactionContent::V3(tx)) => {
            assert!(
                tx.proof_facts.is_none(),
                "proof_facts should not be returned when response_flags are not provided"
            );
        }
        _ => panic!("Expected invoke v3 transaction content without flags"),
    }

    match tx_with_proof_facts {
        TransactionContent::Invoke(InvokeTransactionContent::V3(tx)) => {
            let returned_proof_facts = tx
                .proof_facts
                .expect("proof_facts should be returned when IncludeProofFacts is requested");
            assert_eq!(
                returned_proof_facts, submitted_proof_facts,
                "proof_facts returned by get_block_with_receipts should match sent proof_facts"
            );
        }
        _ => panic!("Expected invoke v3 transaction content with IncludeProofFacts flag"),
    }
}
