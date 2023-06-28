use crate::api::models::transaction::TransactionHashHex;

use super::{
    error::HttpApiError,
    models::{
        AbortedBlocks, AbortingBlocks, Balance, ContractAddress, ContractCode, CreatedBlock,
        FeeToken, ForkStatus, MessageFromL2, MessageHash, MessageToL2, MintTokens,
        MintTokensResponse, Path, PostmanLoadL1MessagingContract, PredeployedAccount, Time,
    },
    HttpApiHandler, HttpApiResult,
};

use axum::{extract::Query, Extension, Json};

pub(crate) async fn is_alive() -> HttpApiResult<()> {
    Ok(())
}

/// Dumping and loading
pub(crate) async fn dump(
    Json(_path): Json<Path>,
    Extension(_state): Extension<HttpApiHandler>,
) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn load(
    Json(_path): Json<Path>,
    Extension(_state): Extension<HttpApiHandler>,
) -> HttpApiResult<()> {
    Err(HttpApiError::PathNotFound)
}

/// Postman
pub(crate) async fn postman_load(
    Json(_l1_contract): Json<PostmanLoadL1MessagingContract>,
) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_flush() -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_send_message_to_l2(
    Json(_data): Json<MessageToL2>,
) -> HttpApiResult<Json<TransactionHashHex>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_consume_message_from_l2(
    Json(_data): Json<MessageFromL2>,
) -> HttpApiResult<Json<MessageHash>> {
    Err(HttpApiError::GeneralError)
}

/// Blocks
pub(crate) async fn create_block() -> HttpApiResult<Json<CreatedBlock>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn abort_blocks(
    Json(_data): Json<AbortingBlocks>,
) -> HttpApiResult<Json<AbortedBlocks>> {
    Err(HttpApiError::GeneralError)
}

/// Restart
pub(crate) async fn retart() -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

/// Time
pub(crate) async fn set_time(Json(_data): Json<Time>) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn increase_time(Json(_data): Json<Time>) -> HttpApiResult<()> {
    Err(HttpApiError::GeneralError)
}

/// Accounts
pub(crate) async fn predeployed_accounts() -> HttpApiResult<Json<Vec<PredeployedAccount>>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn get_contract_code(
    Query(_contract_address): Query<ContractAddress>,
) -> HttpApiResult<Json<ContractCode>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn get_account_balance(
    Query(_contract_address): Query<ContractAddress>,
) -> HttpApiResult<Json<Balance>> {
    Err(HttpApiError::GeneralError)
}

/// Mint token - Local faucet

pub(crate) async fn get_fee_token() -> HttpApiResult<Json<FeeToken>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn mint(Json(_data): Json<MintTokens>) -> HttpApiResult<Json<MintTokensResponse>> {
    Err(HttpApiError::GeneralError)
}

/// Fork
pub(crate) async fn get_fork_status() -> HttpApiResult<Json<ForkStatus>> {
    Err(HttpApiError::GeneralError)
}
