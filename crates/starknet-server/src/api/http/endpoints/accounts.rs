use axum::extract::Query;
use axum::{Extension, Json};
use starknet_types::traits::ToHexString;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{Balance, ContractAddress, PredeployedAccount};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn predeployed_accounts(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<Vec<PredeployedAccount>>> {
    let predeployed_accounts = state
        .api
        .starknet
        .read()
        .await
        .get_predeployed_accounts()
        .into_iter()
        .map(|acc| PredeployedAccount {
            initial_balance: 0,
            address: acc.account_address.to_prefixed_hex_str(),
            public_key: acc.public_key.to_prefixed_hex_str(),
            private_key: acc.private_key.to_prefixed_hex_str(),
        })
        .collect::<Vec<PredeployedAccount>>();

    Ok(Json(predeployed_accounts))
}

pub(crate) async fn get_account_balance(
    Query(_contract_address): Query<ContractAddress>,
) -> HttpApiResult<Json<Balance>> {
    Err(HttpApiError::GeneralError)
}
