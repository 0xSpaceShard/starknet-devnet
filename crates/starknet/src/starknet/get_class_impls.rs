use crate::error::{Error, Result};
use crate::starknet::Starknet;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};

use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::state::state_api::StateReader;
use starknet_rs_core::types::BlockId;

pub fn get_class_hash_at_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> Result<ClassHash> {
    let state_reader = starknet.get_state_reader_at_mut(&block_id)?;
    Ok(state_reader.get_class_hash_at(&contract_address.try_into()?)?.into())
}

pub fn get_class_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    class_hash: ClassHash,
) -> Result<CompiledClass> {
    let state_reader = starknet.get_state_reader_at_mut(&block_id)?;
    Ok(state_reader.get_contract_class(&class_hash.try_into()?)?)
}

pub fn get_class_at_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> Result<CompiledClass> {
    let class_hash = starknet.get_class_hash_at(block_id, contract_address)?;
    starknet.get_class(block_id, class_hash)
}
