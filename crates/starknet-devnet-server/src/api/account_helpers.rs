use serde::Deserialize;
use starknet_core::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
use starknet_core::error::DevnetResult;
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::join_felts;
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::block::{BlockId, BlockTag};
use starknet_types::rpc::transaction_receipt::FeeUnit;

use crate::api::error::ApiError;
use crate::api::models::AccountBalanceResponse;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BalanceQuery {
    pub address: Felt,
    pub unit: Option<FeeUnit>,
    pub block_id: Option<BlockId>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PredeployedAccountsQuery {
    pub with_balance: Option<bool>,
}

pub fn get_balance(
    starknet: &mut Starknet,
    address: ContractAddress,
    erc20_address: ContractAddress,
    block_id: BlockId,
) -> Result<BigUint, ApiError> {
    let balance_selector =
        starknet_rs_core::utils::get_selector_from_name("balanceOf").map_err(|err| {
            starknet_core::error::Error::UnexpectedInternalError { msg: err.to_string() }
        })?;
    let new_balance_raw = starknet.call(
        &block_id,
        erc20_address.into(),
        balance_selector,
        vec![Felt::from(address)], // calldata = the address being queried
    )?;

    match new_balance_raw.as_slice() {
        // format balance for output - initially it is a 2-member vector (low, high)
        [new_balance_low, new_balance_high] => Ok(join_felts(new_balance_high, new_balance_low)),
        _ => {
            let msg = format!(
                "Fee token contract expected to return 2 values; got: {new_balance_raw:?}",
            );

            Err(ApiError::StarknetDevnetError(
                starknet_core::error::Error::UnexpectedInternalError { msg },
            ))
        }
    }
}

/// Returns the address of the ERC20 (fee token) contract associated with the unit.
pub fn get_erc20_address(unit: &FeeUnit) -> DevnetResult<ContractAddress> {
    let erc20_contract_address = match unit {
        FeeUnit::WEI => ETH_ERC20_CONTRACT_ADDRESS,
        FeeUnit::FRI => STRK_ERC20_CONTRACT_ADDRESS,
    };

    Ok(ContractAddress::new(erc20_contract_address)?)
}

pub fn get_balance_unit(
    starknet: &mut Starknet,
    address: ContractAddress,
    unit: FeeUnit,
) -> Result<AccountBalanceResponse, ApiError> {
    let erc20_address =
        get_erc20_address(&unit).map_err(|e| ApiError::InvalidValueError { msg: e.to_string() })?;
    let amount =
        get_balance(starknet, address, erc20_address, BlockId::Tag(BlockTag::PreConfirmed))
            .map_err(|e| ApiError::GeneralError(e.to_string()))?;

    Ok(AccountBalanceResponse { amount: amount.to_string(), unit })
}
