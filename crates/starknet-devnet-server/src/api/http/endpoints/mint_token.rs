use starknet_core::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{BlockId, BlockTag, Felt};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{felt_from_prefixed_hex, join_felts};
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::transaction_receipt::FeeUnit;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{MintTokensRequest, MintTokensResponse};
use crate::api::http::HttpApiResult;
use crate::api::json_rpc::error::ApiError;
use crate::api::Api;

/// get the balance of the `address`
pub fn get_balance(
    starknet: &mut Starknet,
    address: ContractAddress,
    erc20_address: ContractAddress,
    tag: BlockTag,
) -> Result<BigUint, ApiError> {
    let balance_selector =
        starknet_rs_core::utils::get_selector_from_name("balanceOf").map_err(|err| {
            starknet_core::error::Error::UnexpectedInternalError { msg: err.to_string() }
        })?;
    let new_balance_raw = starknet.call(
        &BlockId::Tag(tag),
        erc20_address.into(),
        balance_selector,
        vec![Felt::from(address)], // calldata = the address being queried
    )?;

    match new_balance_raw.as_slice() {
        // format balance for output - initially it is a 2-member vector (low, high)
        [new_balance_low, new_balance_high] => Ok(join_felts(new_balance_high, new_balance_low)),
        _ => {
            let msg = format!(
                "Fee token contract expected to return 2 values; got: {:?}",
                new_balance_raw
            );

            Err(ApiError::ContractError {
                error: starknet_core::error::Error::UnexpectedInternalError { msg },
            })
        }
    }
}

/// Returns the address of the ERC20 (fee token) contract associated with the unit.
// unwraps are safe to use, because those are constants from mainnet
pub fn get_erc20_address(unit: &FeeUnit) -> HttpApiResult<ContractAddress> {
    let erc20_contract_address_string = match unit {
        FeeUnit::WEI => ETH_ERC20_CONTRACT_ADDRESS,
        FeeUnit::FRI => STRK_ERC20_CONTRACT_ADDRESS,
    };

    ContractAddress::new(
        felt_from_prefixed_hex(erc20_contract_address_string)
            .map_err(|err| HttpApiError::InvalidValueError { msg: err.to_string() })?,
    )
    .map_err(|err| HttpApiError::InvalidValueError { msg: err.to_string() })
}

pub(crate) async fn mint_impl(
    api: &Api,
    request: MintTokensRequest,
) -> HttpApiResult<MintTokensResponse> {
    let mut starknet = api.starknet.lock().await;
    let unit = request.unit.unwrap_or(FeeUnit::WEI);
    let erc20_address = get_erc20_address(&unit)?;

    // increase balance
    let tx_hash = starknet
        .mint(request.address, request.amount, erc20_address)
        .await
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    let new_balance = get_balance(&mut starknet, request.address, erc20_address, BlockTag::Pending)
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    Ok(MintTokensResponse { new_balance: new_balance.to_str_radix(10), unit, tx_hash })
}
