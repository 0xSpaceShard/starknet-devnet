use axum::{Extension, Json};
use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::felt::Felt;
use starknet_types::traits::ToDecimalString;

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
    let new_balance = starknet
        .call(
            BlockId::Tag(BlockTag::Pending),
            erc20_address,
            balance_selector,
            vec![Felt::from(request.address.0)], // calldata = the incremented address
        )
        .map_err(|err| HttpApiError::MintingError { msg: err.to_string() })?;

    // format new balance
    assert_eq!(new_balance.len(), 1); // TODO 1 or 2?
    let new_balance = new_balance.get(0).unwrap();

    Ok(Json(MintTokensResponse {
        new_balance: new_balance.to_decimal_string(),
        unit: "WEI".to_string(),
        tx_hash: FeltHex(tx_hash),
    }))
}
