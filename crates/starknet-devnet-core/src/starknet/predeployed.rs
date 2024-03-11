use blockifier::state::state_api::State;
use starknet_rs_core::utils::{get_selector_from_name, get_storage_var_address};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::error::Error::ProgramError;
use starknet_types::felt::Felt;

use crate::constants::{
    CAIRO_1_ERC20_CONTRACT_CLASS_HASH, CAIRO_1_ERC20_CONTRACT_PATH, CHARGEABLE_ACCOUNT_ADDRESS,
    UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_PATH,
};
use crate::error::{DevnetResult, Error};
use crate::state::StarknetState;
use crate::system_contract::SystemContract;

pub(crate) fn create_erc20_at_address(contract_address: &str) -> DevnetResult<SystemContract> {
    let erc20_contract_class_json_str =
        std::fs::read_to_string(CAIRO_1_ERC20_CONTRACT_PATH).map_err(|err| {
            Error::ReadFileError { source: err, path: CAIRO_1_ERC20_CONTRACT_PATH.to_string() }
        })?;
    let erc20_fee_contract = SystemContract::new_cairo1(
        CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
        contract_address,
        &erc20_contract_class_json_str,
    )?;
    Ok(erc20_fee_contract)
}

/// Set initial values of ERC20 contract storage
pub(crate) fn initialize_erc20_at_address(
    state: &mut StarknetState,
    contract_address: &str,
    erc20_name: &str,
    erc20_symbol: &str,
) -> DevnetResult<()> {
    let contract_address = ContractAddress::new(Felt::from_prefixed_hex_str(contract_address)?)?;

    for (storage_var_name, storage_value) in [
        (
            "ERC20_name",
            get_selector_from_name(erc20_name)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?
                .into(),
        ),
        (
            "ERC20_symbol",
            get_selector_from_name(erc20_symbol)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?
                .into(),
        ),
        ("ERC20_decimals", 18.into()),
        // necessary to set - otherwise minting txs cannot be executed
        ("Ownable_owner", Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let storage_var_address = starknet_types::patricia_key::PatriciaKey::new(Felt::new(
            get_storage_var_address(storage_var_name, &[]).map_err(|_| ProgramError)?.to_bytes_be(),
        )?)?;
        state.set_storage_at(
            contract_address.try_into()?,
            storage_var_address,
            storage_value.into(),
        )?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc_contract_class_json_str = std::fs::read_to_string(UDC_CONTRACT_PATH)
        .map_err(|err| Error::ReadFileError { source: err, path: UDC_CONTRACT_PATH.to_string() })?;
    let udc_contract = SystemContract::new_cairo0(
        UDC_CONTRACT_CLASS_HASH,
        UDC_CONTRACT_ADDRESS,
        &udc_contract_class_json_str,
    )?;

    Ok(udc_contract)
}
