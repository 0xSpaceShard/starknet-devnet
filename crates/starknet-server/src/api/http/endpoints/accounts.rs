use axum::extract::Query;
use axum::{Extension, Json};
use starknet_types::contract_address::ContractAddress;
use starknet_types::traits::ToDecimalString;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{Balance, SerializableAccount};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn get_predeployed_accounts(
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
            initial_balance: acc.initial_balance.to_decimal_string(),
            address: acc.account_address,
            public_key: acc.public_key,
            private_key: acc.private_key,
        })
        .collect();

    Ok(Json(predeployed_accounts))
}

pub(crate) async fn get_account_balance(
    Query(_contract_address): Query<ContractAddress>,
) -> HttpApiResult<Json<Balance>> {
    Err(HttpApiError::GeneralError)
}
