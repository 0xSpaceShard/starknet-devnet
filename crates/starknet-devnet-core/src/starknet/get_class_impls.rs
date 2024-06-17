use blockifier::state::state_api::StateReader;
use starknet_rs_core::types::BlockId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;

use crate::error::{DevnetResult, Error, StateError};
use crate::starknet::Starknet;
use crate::state::CustomStateReader;

pub fn get_class_hash_at_impl(
    starknet: &mut Starknet,
    block_id: &BlockId,
    contract_address: ContractAddress,
) -> DevnetResult<ClassHash> {
    let state = starknet.get_mut_state_at(block_id)?;
    let core_address = contract_address.try_into()?;

    let class_hash = state.get_class_hash_at(core_address)?;
    if class_hash == Default::default() {
        Err(Error::ContractNotFound)
    } else {
        Ok(class_hash.into())
    }
}

pub fn get_class_impl(
    starknet: &mut Starknet,
    block_id: &BlockId,
    class_hash: ClassHash,
) -> DevnetResult<ContractClass> {
    // if provided with block hash, convert to number - the underlying logic only works with that or
    // block tag
    let block_id = if let BlockId::Hash(block_hash) = block_id {
        match starknet.blocks.hash_to_block.get(&block_hash.into()) {
            Some(block) => BlockId::Number(block.block_number().0),
            None => return Err(Error::NoBlock),
        }
    } else {
        *block_id
    };

    // TODO do we even need the state at the specific block_id? Perhaps the class storage should be
    // a property of the parent class.
    let state = starknet.get_mut_state_at(&block_id)?;
    match state.get_rpc_contract_class(&class_hash, &block_id) {
        Some(class) => Ok(class.clone()),
        None => Err(Error::StateError(StateError::NoneClassHash(class_hash))),
    }
}

pub fn get_class_at_impl(
    starknet: &mut Starknet,
    block_id: &BlockId,
    contract_address: ContractAddress,
) -> DevnetResult<ContractClass> {
    let class_hash = starknet.get_class_hash_at(block_id, contract_address)?;
    starknet.get_class(block_id, class_hash)
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::BlockId;
    use starknet_types::contract_class::ContractClass;

    use crate::error::Error;
    use crate::starknet::starknet_config::StateArchiveCapacity;
    use crate::starknet::tests::setup_starknet_with_no_signature_check_account_and_state_capacity;
    use crate::utils::test_utils::dummy_broadcasted_declare_transaction_v2;

    #[test]
    fn get_sierra_class() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e8 as u128,
                StateArchiveCapacity::Full,
            );

        let declare_txn = dummy_broadcasted_declare_transaction_v2(&account.account_address);

        let expected: ContractClass = declare_txn.contract_class.clone().into();
        let (_, class_hash) = starknet
            .add_declare_transaction(
                starknet_types::rpc::transactions::BroadcastedDeclareTransaction::V2(Box::new(
                    declare_txn,
                )),
            )
            .unwrap();

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let contract_class =
            starknet.get_class(&BlockId::Number(block_number.0), class_hash).unwrap();

        assert_eq!(contract_class, expected)
    }

    #[test]
    fn get_class_hash_at_generated_accounts() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e8 as u128,
                StateArchiveCapacity::Full,
            );

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let block_id = BlockId::Number(block_number.0);

        let class_hash = starknet.get_class_hash_at(&block_id, account.account_address).unwrap();
        let expected = account.class_hash;
        assert_eq!(class_hash, expected);
    }

    #[test]
    fn get_class_hash_at_generated_accounts_without_state_archive() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e8 as u128,
                StateArchiveCapacity::None,
            );

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let block_id = BlockId::Number(block_number.0);

        let class_hash = starknet.get_class_hash_at(&block_id, account.account_address);
        match class_hash.err().unwrap() {
            Error::NoStateAtBlock { .. } => (),
            _ => panic!("Should fail with NoStateAtBlock."),
        }
    }

    #[test]
    fn get_class_at_generated_accounts() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e8 as u128,
                StateArchiveCapacity::Full,
            );

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let block_id = BlockId::Number(block_number.0);

        let contract_class = starknet.get_class_at(&block_id, account.account_address).unwrap();
        assert_eq!(contract_class, account.contract_class);
    }
}
