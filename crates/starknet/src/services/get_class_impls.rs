use starknet_in_rust::{
    utils::ClassHash,
    state::state_api::StateReader,
};
use crate::Starknet;
use starknet_rs_core::types::BlockId;
use starknet_rs_core::types::contract::CompiledClass;
use starknet_types::{
    contract_class::ContractClass,
    error::{self, Error},
    felt::Felt,
    DevnetResult,
};

pub fn get_class_hash_at_impl(
    starknet: &Starknet,
    block_id: BlockId,
    contract_address: ContractAddressHex,
) -> DevnetResult<ClassHash> {
    let parsed_address = contract_address.0.try_into().unwrap();
    match starknet.state.state.address_to_class_hash.get(&parsed_address) {
        Some(class_hash) => Ok(*class_hash),
        None => Err(error::ApiError::ContractNotFound),
    }
}

pub fn get_class_impl(
    starknet: &Starknet,
    block_id: BlockId,
    class_hash: ClassHashHex,
) -> DevnetResult<CompiledClass> {
    starknet.state.state.get_contract_class(class_hash)
}

pub fn get_class_at_impl(
    starknet: &Starknet,
    block_id: BlockId,
    contract_address: ContractAddressHex,
) -> DevnetResult<ContractClass> {
    let class_hash =
}
