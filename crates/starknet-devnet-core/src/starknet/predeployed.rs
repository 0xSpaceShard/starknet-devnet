use blockifier::state::state_api::State;
use starknet_rs_core::types::Felt;
use starknet_rs_core::utils::cairo_short_string_to_felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::felt_from_prefixed_hex;
use starknet_types_core::hash::Poseidon;

use crate::constants::{
    CHARGEABLE_ACCOUNT_ADDRESS, UDC_CONTRACT, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use crate::error::{DevnetResult, Error};
use crate::state::StarknetState;
use crate::system_contract::SystemContract;
use crate::utils::get_storage_var_address;

pub(crate) fn create_erc20_at_address_extended(
    contract_address: Felt,
    class_hash: Felt,
    contract_class_json_str: &str,
) -> DevnetResult<SystemContract> {
    let erc20_fee_contract =
        SystemContract::new_cairo1(class_hash, contract_address, contract_class_json_str)?;
    Ok(erc20_fee_contract)
}

fn store_short_string_as_byte_array(
    state: &mut StarknetState,
    contract_address: ContractAddress,
    storage_var_name: &str,
    short_str: &str,
) -> DevnetResult<()> {
    let storage_var_address = get_storage_var_address(storage_var_name, &[])?.try_into()?;

    let felt_value =
        cairo_short_string_to_felt(short_str).map_err(|_| Error::UnexpectedInternalError {
            msg: format!("Cannot create a ByteArray from {short_str}"),
        })?;

    state.set_storage_at(
        contract_address.try_into()?,
        storage_var_address,
        short_str.len().into(),
    )?;

    // That's how ByteArray is defined
    let capacity_arg = cairo_short_string_to_felt("ByteArray")
        .map_err(|e| Error::UnexpectedInternalError { msg: e.to_string() })?;

    let chunk_index = Felt::ZERO;
    let mut hashable = [storage_var_address.into(), chunk_index, capacity_arg];
    Poseidon::hades_permutation(&mut hashable);
    let chunk_base = starknet_api::state::StorageKey::try_from(hashable[0])?;

    // no offset in chunk because the word is expected to be short
    state.set_storage_at(contract_address.try_into()?, chunk_base, felt_value)?;

    Ok(())
}

/// Set initial values of ERC20 contract storage
pub(crate) fn initialize_erc20_at_address(
    state: &mut StarknetState,
    contract_address: Felt,
    erc20_name: &str,
    erc20_symbol: &str,
) -> DevnetResult<()> {
    let contract_address = ContractAddress::new(contract_address)?;

    for (storage_var_name, string) in [("ERC20_name", erc20_name), ("ERC20_symbol", erc20_symbol)] {
        store_short_string_as_byte_array(state, contract_address, storage_var_name, string)?;
    }

    for (storage_var_name, storage_value) in [
        ("ERC20_decimals", 18.into()),
        // necessary to set - otherwise minting txs cannot be executed
        ("Ownable_owner", felt_from_prefixed_hex(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let storage_var_address = get_storage_var_address(storage_var_name, &[])?.try_into()?;
        state.set_storage_at(contract_address.try_into()?, storage_var_address, storage_value)?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc_contract =
        SystemContract::new_cairo0(UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT)?;

    Ok(udc_contract)
}

#[cfg(test)]
pub(crate) mod tests {
    use starknet_rs_core::types::Felt;

    use crate::constants::{CAIRO_1_ERC20_CONTRACT, CAIRO_1_ERC20_CONTRACT_CLASS_HASH};
    use crate::error::DevnetResult;
    use crate::system_contract::SystemContract;

    pub(crate) fn create_erc20_at_address(contract_address: Felt) -> DevnetResult<SystemContract> {
        let erc20_fee_contract = SystemContract::new_cairo1(
            CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
            contract_address,
            CAIRO_1_ERC20_CONTRACT,
        )?;
        Ok(erc20_fee_contract)
    }
}
