use axum::{Extension, Json};
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transactions::L1HandlerTransaction;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    FlushParameters, FlushedMessages, MessageHash, MessagingLoadAddress,
    PostmanLoadL1MessagingContract, TxHash,
};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn postman_load(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<PostmanLoadL1MessagingContract>,
) -> HttpApiResult<Json<MessagingLoadAddress>> {
    let mut starknet = state.api.starknet.write().await;

    let messaging_contract_address = starknet
        .configure_messaging(
            &data.network_url,
            data.address.as_deref(),
            data.private_key.as_deref(),
        )
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessagingLoadAddress { messaging_contract_address }))
}

pub(crate) async fn postman_flush(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<FlushParameters>,
) -> HttpApiResult<Json<FlushedMessages>> {
    // Need to handle L1 to L2 first in case that those messages
    // will create L2 to L1 messages.
    let mut starknet = state.api.starknet.write().await;

    let dry_run = data.dry_run.unwrap_or(false);

    let messages_to_l2 = if dry_run {
        vec![]
    } else {
        starknet.fetch_and_execute_messages_to_l2().await.map_err(|e| {
            HttpApiError::MessagingError { msg: format!("messages to l2 error: {}", e) }
        })?
    };

    let from_block = if let Some(m) = &starknet.messaging { m.last_local_block } else { 0 };

    let (messages_to_l1, last_local_block) = if dry_run {
        (
            starknet.collect_messages_to_l1(from_block).await.map_err(|e| {
                HttpApiError::MessagingError { msg: format!("messages to l1 error: {}", e) }
            })?,
            0,
        )
    } else {
        starknet.collect_and_send_messages_to_l1(from_block).await.map_err(|e| {
            HttpApiError::MessagingError { msg: format!("messages to l1 error: {}", e) }
        })?
    };

    let l1_provider = if dry_run {
        "dry run".to_string()
    } else {
        starknet.messaging_url().unwrap_or("Not set".to_string())
    };

    if !dry_run {
        // +1 to ensure this last block is not collected anymore.
        starknet
            .messaging_mut()
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
            .last_local_block = last_local_block + 1;
    }

    Ok(Json(FlushedMessages { messages_to_l1, messages_to_l2, l1_provider }))
}

pub(crate) async fn postman_send_message_to_l2(
    Extension(state): Extension<HttpApiHandler>,
    Json(message): Json<MessageToL2>,
) -> HttpApiResult<Json<TxHash>> {
    let mut starknet = state.api.starknet.write().await;

    let chain_id = starknet.chain_id().to_felt();

    let transaction = L1HandlerTransaction::try_from_message_to_l2(message)
        .map_err(|_| HttpApiError::InvalidValueError {
            msg: "The `paid_fee_on_l1` is out of range, expecting u128 value".to_string(),
        })?
        .with_hash(chain_id);

    let transaction_hash = transaction.transaction_hash;

    starknet
        .add_l1_handler_transaction(transaction)
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(TxHash { transaction_hash }))
}

pub(crate) async fn postman_consume_message_from_l2(
    Extension(state): Extension<HttpApiHandler>,
    Json(message): Json<MessageToL1>,
) -> HttpApiResult<Json<MessageHash>> {
    let starknet = state.api.starknet.read().await;

    let message_hash = starknet
        .messaging_ref()
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
        .consume_mock_message(&message)
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessageHash { message_hash }))
}
