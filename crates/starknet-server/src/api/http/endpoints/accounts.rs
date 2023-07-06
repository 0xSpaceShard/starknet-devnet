use axum::{extract::Query, Json};

use crate::api::http::{
    error::HttpApiError,
    models::{Balance, ContractAddress, PredeployedAccount},
    HttpApiResult,
};

pub(crate) async fn predeployed_accounts() -> HttpApiResult<Json<Vec<PredeployedAccount>>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn get_account_balance(
    Query(_contract_address): Query<ContractAddress>,
) -> HttpApiResult<Json<Balance>> {
    Err(HttpApiError::GeneralError)
}
