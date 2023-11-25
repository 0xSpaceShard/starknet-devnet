use axum::{Extension, Json};
use starknet_rs_core::types::MsgToL1;
use starknet_types::rpc::transactions::L1HandlerTransaction;
use starknet_types::rpc::transaction_receipt::MessageToL1;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    FlushParameters, FlushedMessages, MessageHash, PostmanMessageToL1, PostmanMessageToL2, MessagingLoadAddress,
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
        Ok(vec![])
    } else {
        starknet
            .fetch_and_execute_messages_to_l2()
            .await
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<PostmanMessageToL2>, HttpApiError>>()
    };

    let from_block = if let Some(m) = &starknet.messaging { m.last_local_block } else { 0 };

    let (messages_to_l1, last_local_block) = if dry_run {
        (
            starknet
                .collect_messages_to_l1(from_block)
                .await
                .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
                .into_iter()
                .map(|m| m.into())
                .collect::<Vec<PostmanMessageToL1>>(),
            0,
        )
    } else {
        let (msgs, b) = starknet
            .collect_and_send_messages_to_l1(from_block)
            .await
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

        (
            msgs.into_iter()
                .map(|m| m.into())
                .collect::<Vec<PostmanMessageToL1>>(),
            b,
        )
    };

    let l1_provider = if dry_run {
        "dry run".to_string()
    } else {
        starknet.messaging_url().unwrap_or("Not set".to_string())
    };

    if !dry_run {
        // +1 to ensure this last block is not collected anymore.
        starknet.messaging.as_mut().expect("Messaging expected configured").last_local_block =
            last_local_block + 1;
    }

    Ok(Json(FlushedMessages { messages_to_l1, messages_to_l2, l1_provider }))
}

pub(crate) async fn postman_send_message_to_l2(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<PostmanMessageToL2>,
) -> HttpApiResult<Json<TxHash>> {
    let mut starknet = state.api.starknet.write().await;

    let chain_id = starknet.chain_id().to_felt();

    let transaction = L1HandlerTransaction::try_from(data)?.with_hash(chain_id);
    let transaction_hash = transaction.transaction_hash;

    starknet
        .add_l1_handler_transaction(transaction)
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(TxHash { transaction_hash }))
}

pub(crate) async fn postman_consume_message_from_l2(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<PostmanMessageToL1>,
) -> HttpApiResult<Json<MessageHash>> {
    let starknet = state.api.starknet.read().await;

    let message: MessageToL1 = data.into();

    let message_hash = starknet
        .messaging
        .as_ref()
        .expect("Messaging is not configured.")
        .consume_mock_message(&message)
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessageHash { message_hash }))
}
