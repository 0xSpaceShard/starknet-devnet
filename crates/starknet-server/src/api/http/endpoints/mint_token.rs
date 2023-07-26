use axum::{Extension, Json};
use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{FeeToken, MintTokensRequest, MintTokensResponse};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::models::FeltHex;

pub(crate) async fn get_fee_token() -> HttpApiResult<Json<FeeToken>> {
    Err(HttpApiError::GeneralError)
}

/// get the balance of the `address`
fn get_balance(
    starknet: &Starknet,
    address: ContractAddress,
) -> Result<BigUint, starknet_core::error::Error> {
    let erc20_address = Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS).unwrap();
    let balance_selector =
        starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap().into();
    let new_balance_raw = starknet.call(
        BlockId::Tag(BlockTag::Pending),
        erc20_address,
        balance_selector,
        vec![Felt::from(address)], // calldata = the query balance
    )?;

    // format balance for output - initially it is a 2-member vector (low, high)
    assert_eq!(new_balance_raw.len(), 2);
    let new_balance_low: BigUint = (*new_balance_raw.get(0).unwrap()).into();
    let new_balance_high: BigUint = (*new_balance_raw.get(1).unwrap()).into();
    let new_balance: BigUint = (new_balance_high << 128) + new_balance_low;
    Ok(new_balance)
}

pub(crate) async fn mint(
    Json(request): Json<MintTokensRequest>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<MintTokensResponse>> {
    // increase balance
    let mut starknet = state.api.starknet.write().await;

    // TODO DEBUG
    let old_balance = get_balance(&starknet, request.address.0).unwrap();
    println!("DEBUG old balance: {old_balance}");

    let tx_hash = starknet.mint(request.address.0, request.amount).await.map_err(|err| {
        let new_balance = get_balance(&starknet, request.address.0).unwrap();
        println!("DEBUG new balance after catching error: {new_balance}");
        HttpApiError::MintingError { msg: err.to_string() }
    })?;

    let new_balance = get_balance(&starknet, request.address.0)
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;
    println!("DEBUG new balance after NOT catching error: {new_balance}");

    Ok(Json(MintTokensResponse {
        new_balance: new_balance.to_str_radix(10),
        unit: "WEI".to_string(),
        tx_hash: FeltHex(tx_hash),
    }))
}
