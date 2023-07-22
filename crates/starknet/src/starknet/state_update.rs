use starknet_rs_core::types::BlockId;
use starknet_types::felt::BlockHash;

use super::Starknet;
use crate::error::Result;
use crate::state::state_diff::StateDiff;

pub fn state_update_by_block_id(
    starknet: &Starknet,
    block_id: BlockId,
) -> Result<(BlockHash, StateDiff)> {
    let block = starknet.blocks.get_by_block_id(block_id)?;
    let state_diff =
        starknet.blocks.num_to_state_diff.get(&block.block_number()).cloned().unwrap_or_default();

    Ok((block.block_hash(), state_diff))
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::core::contract_address::compute_casm_class_hash;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::CasmContractClass;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants;
    use crate::starknet::{predeployed, Starknet};
    use crate::state::state_diff::StateDiff;
    use crate::traits::{Accounted, HashIdentifiedMut};
    use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
    use crate::utils::load_cairo_0_contract_class;
    use crate::utils::test_utils::{dummy_cairo_1_contract_class, dummy_felt};

    #[test]
    /// This test checks that the state update is correct after a declare transaction v2.
    /// Then checks that the state update is empty after executing the same declare transaction
    fn correct_state_update_after_declare_transaction_v2() {
        let (mut starknet, sender_address) = setup();
        let contract_class = dummy_cairo_1_contract_class();

        let sierra_class_hash = contract_class.generate_hash().unwrap();
        let casm_contract_class = CasmContractClass::try_from(contract_class.clone()).unwrap();
        let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

        let declare_txn = DeclareTransactionV2::new(
            contract_class,
            compiled_class_hash.clone().into(),
            sender_address,
            100,
            Vec::new(),
            Felt::from(0),
            StarknetChainId::TestNet.to_felt().into(),
        )
        .unwrap();

        // first execute declare v2 transaction
        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn.clone()).unwrap();
        assert_eq!(
            starknet.transactions.get_by_hash_mut(&txn_hash).unwrap().status,
            TransactionStatus::AcceptedOnL2
        );
        let (_, state_diff) = starknet
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

        // check only 3 of the 4 fields, because the inner property has changes to the storage of
        // the ERC20 contract which are hard to be tested correctly, it depends on the fee
        // calculation of starknet_in_rust_library
        assert_eq!(
            state_diff.cairo_0_declared_contracts,
            expected_state_diff.cairo_0_declared_contracts
        );
        assert_eq!(
            state_diff.class_hash_to_compiled_class_hash,
            expected_state_diff.class_hash_to_compiled_class_hash
        );
        assert_eq!(state_diff.declared_contracts, expected_state_diff.declared_contracts);

        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        assert_eq!(
            starknet.transactions.get_by_hash_mut(&txn_hash).unwrap().status,
            TransactionStatus::AcceptedOnL2
        );

        let (_, state_diff) = starknet
            .block_state_update(starknet_rs_core::types::BlockId::Tag(
                starknet_rs_core::types::BlockTag::Latest,
            ))
            .unwrap();

        assert!(state_diff.declared_contracts.is_empty());
        assert!(state_diff.class_hash_to_compiled_class_hash.is_empty());
        assert!(state_diff.cairo_0_declared_contracts.is_empty());
    }

    // Initializes starknet with account_without_validations
    // deployes ERC20 contract
    fn setup() -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = load_cairo_0_contract_class(account_json_path).unwrap();

        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(100000),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class,
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
