use starknet_rs_core::types::BlockId;

use super::Starknet;
use crate::error::DevnetResult;
use crate::state::state_update::StateUpdate;

pub fn state_update_by_block_id(
    starknet: &Starknet,
    block_id: BlockId,
) -> DevnetResult<StateUpdate> {
    let block = starknet.blocks.get_by_block_id(block_id).ok_or(crate::error::Error::NoBlock)?;
    let state_diff =
        starknet.blocks.num_to_state_diff.get(&block.block_number()).cloned().unwrap_or_default();

    Ok(StateUpdate::new(block.block_hash(), state_diff))
}

#[cfg(test)]
mod tests {
    use cairo_lang_starknet::casm_contract_class::CasmContractClass;
    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{compute_casm_class_hash, Cairo0Json, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::state::ThinStateDiff;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::state::state_diff::StateDiff;
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut};
    use crate::utils::test_utils::{dummy_cairo_1_contract_class, dummy_felt};

    #[test]
    /// This test checks that the state update is correct after a declare transaction v2.
    fn correct_state_update_after_declare_transaction_v2() {
        let (mut starknet, sender_address) = setup();
        let contract_class = dummy_cairo_1_contract_class();

        let sierra_class_hash =
            ContractClass::Cairo1(contract_class.clone()).generate_hash().unwrap();
        let casm_contract_class =
            CasmContractClass::from_contract_class(contract_class.clone(), true).unwrap();
        let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

        let declare_txn = BroadcastedDeclareTransactionV2::new(
            &contract_class,
            compiled_class_hash,
            sender_address,
            Fee(4000),
            &Vec::new(),
            Felt::from(0),
            Felt::from(2),
        );

        // first execute declare v2 transaction
        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        let tx = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        let state_update = starknet
            .block_state_update(starknet_rs_core::types::BlockId::Tag(
                starknet_rs_core::types::BlockTag::Latest,
            ))
            .unwrap();

        let state_diff: ThinStateDiff = state_update.state_diff.into();

        let expected_state_diff: ThinStateDiff = StateDiff {
            declared_contracts: vec![compiled_class_hash],
            class_hash_to_compiled_class_hash: vec![(sierra_class_hash, compiled_class_hash)]
                .into_iter()
                .collect(),
            ..Default::default()
        }
        .into();

        // check only 3 of the 4 fields, because the inner property has changes to the storage of
        // the ERC20 contract which are hard to be tested correctly, it depends on the fee
        // calculation of starknet_in_rust_library
        assert_eq!(
            state_diff.deprecated_declared_classes,
            expected_state_diff.deprecated_declared_classes
        );
        assert_eq!(state_diff.declared_classes, expected_state_diff.declared_classes);
    }

    /// Initializes starknet with account_without_validations
    /// deploys ERC20 contract
    fn setup() -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();

        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(100000),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class.into(),
            erc_20_contract.get_address(),
        )
        .unwrap();

        acc.deploy(&mut starknet.state).unwrap();
        acc.set_initial_balance(&mut starknet.state).unwrap();

        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
