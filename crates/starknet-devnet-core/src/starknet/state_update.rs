use starknet_rs_core::types::BlockId;

use super::Starknet;
use crate::error::DevnetResult;
use crate::state::state_update::StateUpdate;

pub fn state_update_by_block_id(
    starknet: &Starknet,
    block_id: &BlockId,
) -> DevnetResult<StateUpdate> {
    let block = starknet.blocks.get_by_block_id(block_id).ok_or(crate::error::Error::NoBlock)?;
    let state_diff =
        starknet.blocks.hash_to_state_diff.get(&block.block_hash()).cloned().unwrap_or_default();

    Ok(StateUpdate::new(block.block_hash(), state_diff))
}

#[cfg(test)]
mod tests {

    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{Felt, TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::state::ThinStateDiff;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::traits::HashProducer;

    use crate::starknet::tests::setup_starknet_with_no_signature_check_account;
    use crate::state::state_diff::StateDiff;
    use crate::traits::HashIdentifiedMut;
    use crate::utils::calculate_casm_hash;
    use crate::utils::test_utils::dummy_cairo_1_contract_class;

    #[test]
    /// This test checks that the state update is correct after a declare transaction v2.
    fn correct_state_update_after_declare_transaction_v2() {
        let (mut starknet, acc) = setup_starknet_with_no_signature_check_account(1e18 as u128);
        let contract_class = dummy_cairo_1_contract_class();

        let sierra_class_hash =
            ContractClass::Cairo1(contract_class.clone()).generate_hash().unwrap();

        let casm_contract_class_json =
            usc::compile_contract(serde_json::to_value(contract_class.clone()).unwrap()).unwrap();

        let compiled_class_hash = calculate_casm_hash(casm_contract_class_json).unwrap();

        let declare_txn = BroadcastedDeclareTransactionV2::new(
            &contract_class,
            compiled_class_hash,
            acc.account_address,
            Fee(400000),
            &Vec::new(),
            Felt::ZERO,
            Felt::TWO,
        );

        // first execute declare v2 transaction
        let (txn_hash, _) = starknet
            .add_declare_transaction(
                starknet_types::rpc::transactions::BroadcastedDeclareTransaction::V2(Box::new(
                    declare_txn,
                )),
            )
            .unwrap();
        let tx = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        let state_update = starknet
            .block_state_update(&starknet_rs_core::types::BlockId::Tag(
                starknet_rs_core::types::BlockTag::Latest,
            ))
            .unwrap();
        let state_diff = state_update.get_state_diff();

        let expected_state_diff: ThinStateDiff = StateDiff {
            declared_contracts: vec![compiled_class_hash],
            class_hash_to_compiled_class_hash: vec![(sierra_class_hash, compiled_class_hash)]
                .into_iter()
                .collect(),
            ..Default::default()
        }
        .into();

        let class_diff = (state_diff.deprecated_declared_classes, state_diff.declared_classes);
        let expected_class_diff =
            (expected_state_diff.deprecated_declared_classes, expected_state_diff.declared_classes);
        assert_eq!(class_diff, expected_class_diff);
    }
}
