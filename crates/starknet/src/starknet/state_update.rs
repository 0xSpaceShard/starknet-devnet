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

    StateUpdate::new(block.block_hash(), state_diff)
}

#[cfg(test)]
mod tests {
    use starknet_api::transaction::Fee;
    use starknet_in_rust::core::contract_address::compute_casm_class_hash;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::CasmContractClass;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0Json, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants;
    use crate::starknet::{predeployed, Starknet};
    use crate::state::state_diff::StateDiff;
    use crate::state::state_update::StateUpdate;
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut};
    use crate::utils::test_utils::{dummy_cairo_1_contract_class, dummy_felt};

    #[test]
    /// This test checks that the state update is correct after a declare transaction v2.
    /// Then checks that the state update is empty after executing the same declare transaction
    fn correct_state_update_after_declare_transaction_v2() {
        let (mut starknet, sender_address) = setup();
        let contract_class = dummy_cairo_1_contract_class();

        let sierra_class_hash =
            ContractClass::Cairo1(contract_class.clone()).generate_hash().unwrap();
        let casm_contract_class =
            CasmContractClass::from_contract_class(contract_class.clone(), true).unwrap();
        let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

        let mut declare_txn = BroadcastedDeclareTransactionV2::new(
            &contract_class,
            compiled_class_hash.clone().into(),
            sender_address,
            Fee(2000),
            &Vec::new(),
            Felt::from(0),
            Felt::from(2),
        );

        // first execute declare v2 transaction
        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn.clone()).unwrap();
        assert_eq!(
            starknet.transactions.get_by_hash_mut(&txn_hash).unwrap().status,
            TransactionStatus::AcceptedOnL2
        );
        let state_update = starknet
            .block_state_update(starknet_rs_core::types::BlockId::Tag(
                starknet_rs_core::types::BlockTag::Latest,
            ))
            .unwrap();

        let expected_state_diff = StateDiff {
            declared_contracts: vec![(compiled_class_hash.clone().into(), casm_contract_class)]
                .into_iter()
                .collect(),
            class_hash_to_compiled_class_hash: vec![(
                sierra_class_hash,
                compiled_class_hash.into(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let expected_state_update = StateUpdate::new(Felt::default(), expected_state_diff).unwrap();

        // check only 3 of the 4 fields, because the inner property has changes to the storage of
        // the ERC20 contract which are hard to be tested correctly, it depends on the fee
        // calculation of starknet_in_rust_library
        assert_eq!(
            state_update.cairo_0_declared_classes,
            expected_state_update.cairo_0_declared_classes
        );
        assert_eq!(state_update.declared_classes, expected_state_update.declared_classes);

        // execute the same transaction, but increment nonce, so new transaction hash could be
        // computed
        declare_txn.common.nonce = Felt::from(1);
        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        assert_eq!(
            starknet.transactions.get_by_hash_mut(&txn_hash).unwrap().status,
            TransactionStatus::AcceptedOnL2
        );

        let state_update = starknet
            .block_state_update(starknet_rs_core::types::BlockId::Tag(
                starknet_rs_core::types::BlockTag::Latest,
            ))
            .unwrap();

        assert!(state_update.declared_classes.is_empty());
        assert!(state_update.cairo_0_declared_classes.is_empty());
    }

    /// Initializes starknet with account_without_validations
    /// deploys ERC20 contract
    fn setup() -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/cairo0/account_without_validations/account.casm"
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

        starknet.state.synchronize_states();
        starknet.block_context = Starknet::get_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            StarknetChainId::TestNet,
        )
        .unwrap();

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
