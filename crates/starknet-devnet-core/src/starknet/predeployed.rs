use blockifier::state::state_api::State;
use starknet_rs_core::utils::cairo_short_string_to_felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;

use crate::constants::{
    CAIRO_1_ERC20_CONTRACT, CAIRO_1_ERC20_CONTRACT_CLASS_HASH, CHARGEABLE_ACCOUNT_ADDRESS,
    UDC_CONTRACT, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use crate::error::{DevnetResult, Error};
use crate::state::StarknetState;
use crate::system_contract::SystemContract;
use crate::utils::get_storage_var_address;

pub(crate) fn create_erc20_at_address(contract_address: &str) -> DevnetResult<SystemContract> {
    let erc20_fee_contract = SystemContract::new_cairo1(
        CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
        contract_address,
        CAIRO_1_ERC20_CONTRACT,
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
            cairo_short_string_to_felt(erc20_name)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?
                .into(),
        ),
        (
            "ERC20_symbol",
            cairo_short_string_to_felt(erc20_symbol)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?
                .into(),
        ),
        ("ERC20_decimals", 18.into()),
        // necessary to set - otherwise minting txs cannot be executed
        ("Ownable_owner", Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let storage_var_address = get_storage_var_address(storage_var_name, &[])?.try_into()?;
        state.set_storage_at(
            contract_address.try_into()?,
            storage_var_address,
            storage_value.into(),
        )?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc_contract =
        SystemContract::new_cairo0(UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT)?;

    Ok(udc_contract)
}
