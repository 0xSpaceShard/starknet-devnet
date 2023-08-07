use crate::error::{Error, Result};
use crate::starknet::Starknet;
use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::state::state_api::StateReader;
use starknet_in_rust::SierraContractClass;
use starknet_rs_core::types::BlockId;

use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
use starknet_types::felt::ClassHash;

pub fn get_class_hash_at_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> Result<ClassHash> {
    let state = starknet.get_state_at_mut(&block_id)?;
    Ok(state.state.get_class_hash_at(&contract_address.try_into()?)?.into())
}

fn get_sierra_class(
    starknet: &mut Starknet,
    class_hash: &ClassHash,
) -> Result<SierraContractClass> {
    match starknet.state.contract_classes.get(class_hash) {
        Some(contract) => Ok(contract.clone().try_into()?),
        None => Err(Error::FormatError),
    }
}

fn get_cairo_0_class(
    starknet: &mut Starknet,
    class_hash: &ClassHash,
) -> Result<Cairo0ContractClass> {
    match starknet.state.contract_classes.get(class_hash) {
        Some(contract) => Ok(contract.clone().try_into()?),
        None => Err(Error::FormatError),
    }
}

pub fn get_class_impl(
    starknet: &mut Starknet,
    block_id: BlockId,
    class_hash: ClassHash,
) -> Result<ContractClass> {
    let state = starknet.get_state_at_mut(&block_id)?;

    match state.state.get_contract_class(&class_hash.into()) {
        Ok(compiled_class) => match compiled_class {
            CompiledClass::Casm(_) => Ok(get_sierra_class(starknet, &class_hash)?.into()),
            CompiledClass::Deprecated(_) => Ok(get_cairo_0_class(starknet, &class_hash)?.into()),
        },
        Err(err) => Err(err.into()),
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
