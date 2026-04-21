use blockifier::state::state_api::State;
use starknet_rs_core::types::Felt;
use starknet_rs_core::utils::{cairo_short_string_to_felt, normalize_address};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::felt_from_prefixed_hex;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types_core::hash::Poseidon;

use crate::constants::{
    CHARGEABLE_ACCOUNT_ADDRESS, UDC_CONTRACT, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
    UDC_LEGACY_CONTRACT, UDC_LEGACY_CONTRACT_ADDRESS, UDC_LEGACY_CONTRACT_CLASS_HASH,
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

pub(crate) fn initialize_erc20_at_address(
    state: &mut StarknetState,
    contract_address: Felt,
    erc20_name: &str,
    erc20_symbol: &str,
) -> DevnetResult<()> {
    let contract_address = ContractAddress::new(contract_address)?;

    for (storage_var_name, s) in [("ERC20_name", erc20_name), ("ERC20_symbol", erc20_symbol)] {
        assert!(s.len() <= 30, "ByteArray short-string init only supports <= 30 bytes");

        let base: Felt = get_storage_var_address(storage_var_name, &[])?.to_felt();
        let pending_word = cairo_short_string_to_felt(s)
            .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?;

        state.set_storage_at(
            contract_address.into(),
            PatriciaKey::new(base)?.into(),
            Felt::from(s.len() as u64),
        )?;

        let byte_array_marker = cairo_short_string_to_felt("ByteArray")
            .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?;
        let mut state_arr = [base, Felt::ZERO, byte_array_marker];
        Poseidon::hades_permutation(&mut state_arr);
        let chunk_base = normalize_address(state_arr[0]);

        state.set_storage_at(
            contract_address.into(),
            PatriciaKey::new(chunk_base)?.into(),
            pending_word,
        )?;
    }

    for (storage_var_name, storage_value) in [
        ("ERC20_decimals", 18u64.into()),
        ("permitted_minter", felt_from_prefixed_hex(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let key = get_storage_var_address(storage_var_name, &[])?;
        state.set_storage_at(contract_address.into(), key.into(), storage_value)?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc =
        SystemContract::new_cairo1(UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT)?;

    Ok(udc)
}

pub(crate) fn create_legacy_udc() -> DevnetResult<SystemContract> {
    let udc = SystemContract::new_cairo0(
        UDC_LEGACY_CONTRACT_CLASS_HASH,
        UDC_LEGACY_CONTRACT_ADDRESS,
        UDC_LEGACY_CONTRACT,
    )?;

    Ok(udc)
}

#[cfg(test)]
pub(crate) mod tests {
    use starknet_rs_core::types::Felt;

    use crate::constants::{STRK_ERC20_CONTRACT_CLASS, STRK_ERC20_CONTRACT_CLASS_HASH};
    use crate::error::DevnetResult;
    use crate::system_contract::SystemContract;

    pub(crate) fn create_erc20_at_address(contract_address: Felt) -> DevnetResult<SystemContract> {
        let erc20_fee_contract = SystemContract::new_cairo1(
            STRK_ERC20_CONTRACT_CLASS_HASH,
            contract_address,
            STRK_ERC20_CONTRACT_CLASS,
        )?;
        Ok(erc20_fee_contract)
    }
}
