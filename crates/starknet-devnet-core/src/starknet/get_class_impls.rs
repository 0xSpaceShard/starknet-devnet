use blockifier::state::state_api::StateReader;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use starknet_api::block::BlockStatus;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::compile_sierra_contract;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;

use crate::error::{DevnetResult, Error, StateError};
use crate::starknet::Starknet;
use crate::state::BlockNumberOrPending;

pub fn get_class_hash_at_impl(
    starknet: &mut Starknet,
    block_id: &BlockId,
    contract_address: ContractAddress,
) -> DevnetResult<ClassHash> {
    let state = starknet.get_mut_state_at(block_id)?;
    let core_address = contract_address.try_into()?;

    let class_hash = state.get_class_hash_at(core_address)?;
    if class_hash == Default::default() { Err(Error::ContractNotFound) } else { Ok(class_hash.0) }
}

pub fn get_class_impl(
    starknet: &Starknet,
    block_id: &BlockId,
    class_hash: ClassHash,
) -> DevnetResult<ContractClass> {
    let requested_block = starknet.get_block(block_id)?;

    // the underlying logic only works with block number or pending tag
    let block_number_or_pending = match requested_block.status {
        BlockStatus::Pending => BlockNumberOrPending::Pending,
        BlockStatus::AcceptedOnL2 | BlockStatus::AcceptedOnL1 => {
            BlockNumberOrPending::Number(requested_block.block_number().0)
        }
        BlockStatus::Rejected => return Err(Error::NoBlock),
    };

    // Returns sierra for cairo1; returns the only artifact for cairo0.
    match starknet.rpc_contract_classes.read().get_class(&class_hash, &block_number_or_pending) {
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

pub fn get_compiled_casm_impl(
    starknet: &Starknet,
    class_hash: ClassHash,
) -> DevnetResult<CasmContractClass> {
    let contract_class = get_class_impl(starknet, &BlockId::Tag(BlockTag::Latest), class_hash)?;
    match contract_class {
        ContractClass::Cairo1(sierra_contract_class) => {
            let mut casm = compile_sierra_contract(&sierra_contract_class)?;
            casm.pythonic_hints = None; // removes the extra key from serialized form
            Ok(casm)
        }
        ContractClass::Cairo0(_) => Err(Error::StateError(StateError::NoneCasmClass(class_hash))),
    }
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::{BlockId, Felt};
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::transactions::BroadcastedDeclareTransaction;

    use crate::error::Error;
    use crate::starknet::starknet_config::StateArchiveCapacity;
    use crate::starknet::tests::setup_starknet_with_no_signature_check_account_and_state_capacity;
    use crate::utils::test_utils::{
        broadcasted_declare_tx_v3_of_dummy_class, resource_bounds_with_price_1,
    };

    #[test]
    fn get_sierra_class() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e18 as u128,
                StateArchiveCapacity::Full,
            );

        let declare_txn = broadcasted_declare_tx_v3_of_dummy_class(
            account.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        let expected: ContractClass = declare_txn.contract_class.clone().into();
        let (_, class_hash) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(declare_txn)))
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
        starknet.create_block().unwrap(); // makes the queried block non-latest (and unsupported)

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

    #[test]
    fn attempt_getting_class_from_block_before_declaration() {
        let (mut starknet, account) =
            setup_starknet_with_no_signature_check_account_and_state_capacity(
                1e8 as u128,
                StateArchiveCapacity::Full,
            );

        let block_number = starknet.get_latest_block().unwrap().block_number();
        // class not present before the latest block
        let block_id = BlockId::Number(block_number.0 - 1);

        match starknet.get_class_at(&block_id, account.account_address) {
            Err(Error::ContractNotFound) => (),
            other => panic!("Got unexpected resp: {other:?}"),
        }
    }
}
