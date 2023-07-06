use axum::Json;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{FeeToken, MintTokens, MintTokensResponse};
use crate::api::http::HttpApiResult;

pub(crate) async fn get_fee_token() -> HttpApiResult<Json<FeeToken>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn mint(Json(_data): Json<MintTokens>) -> HttpApiResult<Json<MintTokensResponse>> {
    Err(HttpApiError::GeneralError)
}
