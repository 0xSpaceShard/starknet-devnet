use axum::{Extension, Json};
use starknet_rs_core::types::BlockId;
use starknet_types::felt::TransactionHash;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    MessageFromL2, MessageHash, MessageToL2, PostmanLoadL1MessagingContract,
};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn postman_load(
    Json(_l1_contract): Json<PostmanLoadL1MessagingContract>,
) -> HttpApiResult<()> {
    // TODO: we need here to be able to initialize a new EthereumMessaging.
    // However, the private key will not be sent by HTTP,
    // but surely loaded from env / CLI at devnet startup.
    // Hence, we need to add a configuration
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_flush(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<()> {
    // Need to handle L1 to L2 first in case that those messages
    // will create L2 to L1 messages.
    let mut starknet = state.api.starknet.write().await;

    starknet
        .fetch_and_execute_messages_to_l2()
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    // TODO: we need to keep track of the last block id processed
    // locally. To include in the rework of the messaging location
    // in devnet.
    starknet
        .collect_and_send_messages_to_l1(BlockId::Number(0))
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    // TODO: do we also want to consume on L1..? Or only registering the message
    // as being consumable is ok? And then the use can use `consume_message_from_l2`.

    Ok(())
}

pub(crate) async fn postman_send_message_to_l2(
    Json(_data): Json<MessageToL2>,
) -> HttpApiResult<Json<TransactionHash>> {
    // TODO: get info from the model and use add_l1_handler_transaction.
    Err(HttpApiError::GeneralError)
}

pub(crate) async fn postman_consume_message_from_l2(
    Json(_data): Json<MessageFromL2>,
) -> HttpApiResult<Json<MessageHash>> {
    // TODO: call ethereum to consume the message from L2 with the mock function.
    // Do we have to do something on L2 though...?
    Err(HttpApiError::GeneralError)
}
