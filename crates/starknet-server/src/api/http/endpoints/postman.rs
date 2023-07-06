use axum::Json;

use crate::api::{
    http::{
        error::HttpApiError,
        models::{MessageFromL2, MessageHash, MessageToL2, PostmanLoadL1MessagingContract},
        HttpApiResult,
    },
    models::transaction::TransactionHashHex,
};

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
