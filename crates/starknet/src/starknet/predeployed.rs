use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::Felt;

use crate::constants::{
    CHARGEABLE_ACCOUNT_ADDRESS, ERC20_CONTRACT_ADDRESS, ERC20_CONTRACT_CLASS_HASH,
    ERC20_CONTRACT_PATH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_PATH,
};
use crate::error::{DevnetResult, Error};
use crate::state::StarknetState;
use crate::system_contract::SystemContract;
use crate::traits::StateChanger;
use crate::utils::get_storage_var_address;

pub(crate) fn create_erc20() -> DevnetResult<SystemContract> {
    let erc20_contract_class_json_str =
        std::fs::read_to_string(ERC20_CONTRACT_PATH).map_err(|err| Error::ReadFileError {
            source: err,
            path: ERC20_CONTRACT_PATH.to_string(),
        })?;
    let erc20_fee_contract = SystemContract::new(
        ERC20_CONTRACT_CLASS_HASH,
        ERC20_CONTRACT_ADDRESS,
        &erc20_contract_class_json_str,
    )?;

    Ok(erc20_fee_contract)
}

/// Set initial values of ERC20 contract storage
pub(crate) fn initialize_erc20(state: &mut StarknetState) -> DevnetResult<()> {
    let contract_address =
        ContractAddress::new(Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS)?)?;

    for (storage_var_name, storage_value) in [
        ("ERC20_name", get_selector_from_name("ether").unwrap().into()),
        ("ERC20_symbol", get_selector_from_name("ETH").unwrap().into()),
        ("ERC20_decimals", 18.into()),
        // necessary to set - otherwise minting txs cannot be executed
        ("Ownable_owner", Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let storage_var_address = get_storage_var_address(storage_var_name, &[])?;
        let storage_key = ContractStorageKey::new(contract_address, storage_var_address);
        state.change_storage(storage_key, storage_value)?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc_contract_class_json_str = std::fs::read_to_string(UDC_CONTRACT_PATH)
        .map_err(|err| Error::ReadFileError { source: err, path: UDC_CONTRACT_PATH.to_string() })?;
    let udc_contract = SystemContract::new(
        UDC_CONTRACT_CLASS_HASH,
        UDC_CONTRACT_ADDRESS,
        &udc_contract_class_json_str,
    )?;

    Ok(udc_contract)
}
