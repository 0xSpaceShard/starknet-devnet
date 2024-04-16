use axum::extract::Query;
use axum::{Extension, Json};
use starknet_rs_core::types::BlockTag;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::rpc::transaction_receipt::FeeUnit;

use super::mint_token::{get_balance, get_erc20_address};
use crate::api::http::error::HttpApiError;
use crate::api::http::models::{Balance, SerializableAccount};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub async fn get_predeployed_accounts(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<Vec<SerializableAccount>>> {
    let predeployed_accounts = state
        .api
        .starknet
        .read()
        .await
        .get_predeployed_accounts()
        .into_iter()
        .map(|acc| SerializableAccount {
            initial_balance: acc.initial_balance.to_string(),
            address: acc.account_address,
            public_key: acc.public_key,
            private_key: acc.private_key,
        })
        .collect();

    Ok(Json(predeployed_accounts))
}

#[derive(serde::Deserialize, Debug)]
pub struct BalanceQuery {
    address: Felt,
    unit: Option<FeeUnit>,
    pending_block: bool,
}

pub async fn get_account_balance(
    Extension(state): Extension<HttpApiHandler>,
    Query(params): Query<BalanceQuery>,
) -> HttpApiResult<Json<Balance>> {
    // TODO: check here!!!
    let account_address = ContractAddress::new(params.address)
        .map_err(|e| HttpApiError::InvalidValueError { msg: e.to_string() })?;
    let unit = params.unit.unwrap_or(FeeUnit::WEI);
    let erc20_address = get_erc20_address(&unit);
    let block_tag = if params.pending_block { BlockTag::Pending } else { BlockTag::Latest };
    println!("block_tag: {:?}", block_tag);

    let mut starknet = state.api.starknet.write().await;

    let amount = get_balance(&mut starknet, account_address, erc20_address, block_tag)
        .map_err(|e| HttpApiError::GeneralError(e.to_string()))?;
    Ok(Json(Balance { amount, unit }))
}
