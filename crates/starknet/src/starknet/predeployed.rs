use crate::constants::{
    ERC20_CONTRACT_ADDRESS, ERC20_CONTRACT_CLASS_HASH, ERC20_CONTRACT_PATH, UDC_CONTRACT_ADDRESS,
    UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_PATH,
};
use crate::error::{Error, Result};
use crate::system_contract::SystemContract;

pub(crate) fn create_erc20() -> Result<SystemContract> {
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

pub(crate) fn create_udc() -> Result<SystemContract> {
    let udc_contract_class_json_str = std::fs::read_to_string(UDC_CONTRACT_PATH)
        .map_err(|err| Error::ReadFileError { source: err, path: UDC_CONTRACT_PATH.to_string() })?;
    let udc_contract = SystemContract::new(
        UDC_CONTRACT_CLASS_HASH,
        UDC_CONTRACT_ADDRESS,
        &udc_contract_class_json_str,
    )?;

    Ok(udc_contract)
}
