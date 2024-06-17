use axum::extract::State;
use axum::Json;
use starknet_core::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::transaction_receipt::FeeUnit;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{MintTokensRequest, MintTokensResponse};
use crate::api::http::{HttpApiHandler, HttpApiResult};
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
        starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap().into();
    let new_balance_raw = starknet.call(
        &BlockId::Tag(tag),
        erc20_address.into(),
        balance_selector,
        vec![Felt::from(address)], // calldata = the address being queried
    )?;

    // format balance for output - initially it is a 2-member vector (low, high)
    if new_balance_raw.len() != 2 {
        let msg =
            format!("Fee token contract expected to return 2 values; got: {:?}", new_balance_raw);

        return Err(ApiError::ContractError {
            error: starknet_core::error::Error::UnexpectedInternalError { msg },
        });
    }
    let new_balance_low: BigUint = (*new_balance_raw.get(0).unwrap()).into();
    let new_balance_high: BigUint = (*new_balance_raw.get(1).unwrap()).into();
    let new_balance: BigUint = (new_balance_high << 128) + new_balance_low;
    Ok(new_balance)
}

/// Returns the address of the ERC20 (fee token) contract associated with the unit.
pub fn get_erc20_address(unit: &FeeUnit) -> ContractAddress {
    match unit {
        FeeUnit::WEI => {
            ContractAddress::new(Felt::from_prefixed_hex_str(ETH_ERC20_CONTRACT_ADDRESS).unwrap())
                .unwrap()
        }
        FeeUnit::FRI => {
            ContractAddress::new(Felt::from_prefixed_hex_str(STRK_ERC20_CONTRACT_ADDRESS).unwrap())
                .unwrap()
        }
    }
}

pub async fn mint(
    State(state): State<HttpApiHandler>,
    Json(request): Json<MintTokensRequest>,
) -> HttpApiResult<Json<MintTokensResponse>> {
    mint_impl(&state.api, request).await.map(Json::from)
}

pub(crate) async fn mint_impl(
    api: &Api,
    request: MintTokensRequest,
) -> HttpApiResult<MintTokensResponse> {
    let mut starknet = api.starknet.write().await;
    let unit = request.unit.unwrap_or(FeeUnit::WEI);
    let erc20_address = get_erc20_address(&unit);

    // increase balance
    let tx_hash = starknet
        .mint(request.address, request.amount, erc20_address)
        .await
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    let block_tag =
        if starknet.config.blocks_on_demand { BlockTag::Pending } else { BlockTag::Latest };

    let new_balance = get_balance(&mut starknet, request.address, erc20_address, block_tag)
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    Ok(MintTokensResponse { new_balance: new_balance.to_str_radix(10), unit, tx_hash })
}
