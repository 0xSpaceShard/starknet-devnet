use axum::{Extension, Json};
use starknet_rs_core::types::{BlockId, MsgToL1};
use starknet_types::rpc::transactions::L1HandlerTransaction;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    FlushParameters, FlushedMessages, MessageHash, MessageToL1, MessageToL2,
    PostmanLoadL1MessagingContract, TxHash,
};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn postman_load(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<PostmanLoadL1MessagingContract>,
) -> HttpApiResult<()> {
    let mut starknet = state.api.starknet.write().await;

    starknet
        .configure_messaging(
            &data.network_url,
            data.address.as_deref(),
            data.private_key.as_deref(),
        )
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(())
}

pub(crate) async fn postman_flush(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<FlushParameters>,
) -> HttpApiResult<Json<FlushedMessages>> {
    // Need to handle L1 to L2 first in case that those messages
    // will create L2 to L1 messages.
    let mut starknet = state.api.starknet.write().await;

    let messages_to_l2 = if data.dry_run {
        Ok(vec![])
    } else {
        starknet
            .fetch_and_execute_messages_to_l2()
            .await
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<MessageToL2>, HttpApiError>>()
    };

    // TODO: we need to keep track of the last block id processed
    // locally. To include in the rework of the messaging location
    // in devnet.
    let messages_to_l1 = if data.dry_run {
        starknet
            .collect_messages_to_l1(BlockId::Number(0))
            .await
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<MessageToL1>, HttpApiError>>()
    } else {
        starknet
            .collect_and_send_messages_to_l1(BlockId::Number(0))
            .await
            .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<MessageToL1>, HttpApiError>>()
    };

    let l1_provider = if data.dry_run {
        "dry run".to_string()
    } else {
        starknet.messaging_url().unwrap_or("Not set".to_string())
    };

    match (messages_to_l1, messages_to_l2) {
        (Ok(l1s), Ok(l2s)) => {
            Ok(Json(FlushedMessages { messages_to_l1: l1s, messages_to_l2: l2s, l1_provider }))
        }
        (Ok(l1s), Err(e)) => {
            if data.dry_run {
                Ok(Json(FlushedMessages {
                    messages_to_l1: l1s,
                    messages_to_l2: vec![],
                    l1_provider,
                }))
            } else {
                Err(HttpApiError::MessagingError { msg: format!("MessagesToL2: {}", e) })
            }
        }
        (Err(e), Ok(l2s)) => {
            if data.dry_run {
                Ok(Json(FlushedMessages {
                    messages_to_l1: vec![],
                    messages_to_l2: l2s,
                    l1_provider,
                }))
            } else {
                Err(HttpApiError::MessagingError { msg: format!("MessagesToL1: {}", e) })
            }
        }
        (Err(e1), Err(e2)) => Err(HttpApiError::MessagingError {
            msg: format!("MessagesToL1: {} || MessagesToL2: {}", e1, e2),
        }),
    }
}

pub(crate) async fn postman_send_message_to_l2(
    Extension(state): Extension<HttpApiHandler>,
    Json(data): Json<MessageToL2>,
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
    Json(data): Json<MessageToL1>,
) -> HttpApiResult<Json<MessageHash>> {
    let starknet = state.api.starknet.read().await;

    let message: MsgToL1 = data.into();

    let message_hash = starknet
        .messaging
        .as_ref()
        .expect("Messaging is not configured.")
        .consume_mock_message(&message)
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessageHash { message_hash }))
}
