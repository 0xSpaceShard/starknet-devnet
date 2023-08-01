use crate::error::{Error, Result};
use crate::starknet::Starknet;
use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::state::state_api::StateReader;
use starknet_rs_core::types::BlockId;

use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};

pub fn get_class_hash_at_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> Result<ClassHash> {
    let state = starknet.get_state_at_mut(&block_id)?;
    Ok(state.state.get_class_hash_at(&contract_address.try_into()?)?.into())
}

pub fn get_class_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    class_hash: ClassHash,
) -> Result<ContractClass> {
    match starknet.sierra_contracts.get(&class_hash) {
        Some(class) => Ok(class.clone()),
        None => Err(Error::FormatError),
    }
}

pub fn get_class_at_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> Result<ContractClass> {
    let class_hash = starknet.get_class_hash_at(block_id, contract_address)?;
    starknet.get_class(block_id, class_hash)
}
