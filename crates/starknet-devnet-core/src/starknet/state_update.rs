use starknet_types::rpc::block::BlockId;
use starknet_types::rpc::state::StateUpdate;

use super::Starknet;
use crate::error::DevnetResult;

pub fn state_update_by_block_id(
    starknet: &Starknet,
    block_id: &BlockId,
) -> DevnetResult<StateUpdate> {
    let block = starknet.blocks.get_by_block_id(block_id).ok_or(crate::error::Error::NoBlock)?;
    let state_diff =
        starknet.blocks.hash_to_state_diff.get(&block.block_hash()).cloned().unwrap_or_default();

    Ok(StateUpdate::new(block.block_hash(), state_diff.into()))
}

#[cfg(test)]
mod tests {
    use starknet_api::contract_class::compiled_class_hash::{HashVersion, HashableCompiledClass};
    use starknet_rs_core::types::{Felt, TransactionExecutionStatus};
    use starknet_types::compile_sierra_contract;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::block::{BlockId, BlockTag};
    use starknet_types::rpc::state::{ClassHashPair, ContractNonce, ThinStateDiff};
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, TransactionFinalityStatus,
    };
    use starknet_types::traits::TryHashProducer;

    use crate::starknet::tests::setup_starknet_with_no_signature_check_account;
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::{
        broadcasted_declare_tx_v3, dummy_cairo_1_contract_class, resource_bounds_with_price_1,
    };

    #[test]
    /// This test checks that the state update is correct after a declare transaction v3.
    fn correct_state_update_after_declare_transaction_v3() {
        let (mut starknet, acc) = setup_starknet_with_no_signature_check_account(1e18 as u128);
        let contract_class = dummy_cairo_1_contract_class();
        let compiled_class_hash =
            compile_sierra_contract(&contract_class).unwrap().hash(&HashVersion::V2).0;

        let declare_txn = broadcasted_declare_tx_v3(
            acc.account_address,
            Felt::ZERO,
            contract_class.clone(),
            compiled_class_hash,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        // first execute declare v3 transaction
        let (txn_hash, _) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(declare_txn)))
            .unwrap();
        let tx = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        let state_update =
            starknet.block_state_update(&BlockId::Tag(BlockTag::Latest), None).unwrap();
        let mut state_diff = state_update.get_state_diff().clone();
        state_diff.storage_diffs.clear(); // too complicated to compare

        let sierra_class_hash = ContractClass::Cairo1(contract_class).try_generate_hash().unwrap();
        let expected_state_diff = ThinStateDiff {
            declared_classes: vec![ClassHashPair {
                class_hash: sierra_class_hash,
                compiled_class_hash,
            }],
            nonces: vec![ContractNonce { contract_address: acc.account_address, nonce: Felt::ONE }],
            ..Default::default()
        };

        assert_eq!(expected_state_diff, state_diff);
    }

    #[test]
    /// Filtering by contract address should only return state diff entries related to that address.
    fn state_update_filtered_by_contract_address() {
        let (mut starknet, acc) = setup_starknet_with_no_signature_check_account(1e18 as u128);
        let contract_class = dummy_cairo_1_contract_class();
        let compiled_class_hash =
            compile_sierra_contract(&contract_class).unwrap().hash(&HashVersion::V2).0;

        let declare_txn = broadcasted_declare_tx_v3(
            acc.account_address,
            Felt::ZERO,
            contract_class.clone(),
            compiled_class_hash,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        let (txn_hash, _) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(declare_txn)))
            .unwrap();
        let tx = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // Without filter: full state diff
        let unfiltered =
            starknet.block_state_update(&BlockId::Tag(BlockTag::Latest), None).unwrap();
        let full_diff = unfiltered.get_state_diff();
        // The declare tx should produce a nonce entry for the account address
        assert!(!full_diff.nonces.is_empty(), "Full state diff should have nonce entries");

        // Filter by the sender address: should keep the account's nonce
        let filtered_by_sender = starknet
            .block_state_update(&BlockId::Tag(BlockTag::Latest), Some(acc.account_address))
            .unwrap();
        let sender_diff = filtered_by_sender.get_state_diff();
        assert!(
            sender_diff.nonces.iter().all(|n| n.contract_address == acc.account_address),
            "Filtered diff should only contain nonces for the sender address"
        );
        assert_eq!(sender_diff.nonces.len(), 1);
        assert_eq!(sender_diff.nonces[0].nonce, Felt::ONE);
        // declared_classes are not address-specific, so they remain in the filtered diff
        assert_eq!(sender_diff.declared_classes.len(), full_diff.declared_classes.len());

        // Filter by an unrelated address: nonces, deployed_contracts, replaced_classes should be
        // empty
        let unrelated_address = ContractAddress::new(Felt::from_hex_unchecked("0xDEAD")).unwrap();
        let filtered_by_other = starknet
            .block_state_update(&BlockId::Tag(BlockTag::Latest), Some(unrelated_address))
            .unwrap();
        let other_diff = filtered_by_other.get_state_diff();
        assert!(other_diff.nonces.is_empty(), "Unrelated address should have no nonces");
        assert!(
            other_diff.deployed_contracts.is_empty(),
            "Unrelated address should have no deployed contracts"
        );
        assert!(
            other_diff.storage_diffs.is_empty()
                || other_diff.storage_diffs.iter().all(|s| s.storage_entries.is_empty()),
            "Unrelated address should have no storage diffs"
        );
        assert!(
            other_diff.replaced_classes.is_empty(),
            "Unrelated address should have no replaced classes"
        );
    }
}
