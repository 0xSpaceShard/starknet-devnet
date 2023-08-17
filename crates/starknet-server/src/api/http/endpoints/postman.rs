use axum::Json;
use starknet_types::felt::TransactionHash;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    MessageFromL2, MessageHash, MessageToL2, PostmanLoadL1MessagingContract,
};
use crate::api::http::HttpApiResult;

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
) -> HttpApiResult<Json<TransactionHash>> {
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_consume_message_from_l2(
    Json(_data): Json<MessageFromL2>,
) -> HttpApiResult<Json<MessageHash>> {
    Err(HttpApiError::GeneralError)
}
