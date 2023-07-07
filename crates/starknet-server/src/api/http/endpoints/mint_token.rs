use axum::Json;

use crate::api::http::{
    error::HttpApiError,
    models::{FeeToken, MintTokens, MintTokensResponse},
    HttpApiResult,
};

pub(crate) async fn get_fee_token() -> HttpApiResult<Json<FeeToken>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn mint(Json(_data): Json<MintTokens>) -> HttpApiResult<Json<MintTokensResponse>> {
    Err(HttpApiError::GeneralError)
}
