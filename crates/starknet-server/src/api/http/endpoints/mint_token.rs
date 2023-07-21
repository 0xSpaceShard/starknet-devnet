use axum::{Extension, Json};
use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{FeeToken, MintTokensRequest, MintTokensResponse};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::models::FeltHex;

pub(crate) async fn get_fee_token() -> HttpApiResult<Json<FeeToken>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn mint(
    Json(request): Json<MintTokensRequest>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<MintTokensResponse>> {
    // increase balance
    let mut starknet = state.api.starknet.write().await;
    let tx_hash = starknet
        .mint(request.address.0, request.amount)
        .await
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    // get new balance
    let erc20_address = Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS).unwrap();
    let balance_selector =
        starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap().into();
    let new_balance_raw = starknet
        .call(
            BlockId::Tag(BlockTag::Pending),
            erc20_address,
            balance_selector,
            vec![Felt::from(request.address.0)], // calldata = the incremented address
        )
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    // format new balance for output - initially it is a 2-member vector (high, low) // TODO check
    // endianness
    assert_eq!(new_balance_raw.len(), 2);
    println!("DEBUG new_balance_raw: {new_balance_raw:?}");
    let new_balance_high: BigUint = (*new_balance_raw.get(1).unwrap()).into();
    let new_balance_low: BigUint = (*new_balance_raw.get(0).unwrap()).into();
    let new_balance: BigUint = (new_balance_high << 128) + new_balance_low;

    Ok(Json(MintTokensResponse {
        new_balance: new_balance.to_str_radix(10),
        unit: "WEI".to_string(),
        tx_hash: FeltHex(tx_hash),
    }))
}
