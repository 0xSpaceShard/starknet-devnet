use starknet_rs_core::types::BlockId;
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
    use starknet_rs_core::types::{
        BlockId, BlockTag, Felt, TransactionExecutionStatus, TransactionFinalityStatus,
    };
    use starknet_types::compile_sierra_contract;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::state::{ClassHashPair, ContractNonce, ThinStateDiff};
    use starknet_types::rpc::transactions::BroadcastedDeclareTransaction;
    use starknet_types::traits::HashProducer;

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
            compile_sierra_contract(&contract_class).unwrap().compiled_class_hash();

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

        let state_update = starknet.block_state_update(&BlockId::Tag(BlockTag::Latest)).unwrap();
        let mut state_diff = state_update.get_state_diff().clone();
        state_diff.storage_diffs.clear(); // too complicated to compare

        let sierra_class_hash = ContractClass::Cairo1(contract_class).generate_hash().unwrap();
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
}
