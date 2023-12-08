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
        .configure_messaging(&data.network_url, data.address.as_deref())
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessagingLoadAddress { messaging_contract_address }))
}

pub(crate) async fn postman_flush(
    Extension(state): Extension<HttpApiHandler>,
    data: Option<Json<FlushParameters>>,
) -> HttpApiResult<Json<FlushedMessages>> {
    // Need to handle L1 to L2 first in case that those messages
    // will create L2 to L1 messages.
    let mut starknet = state.api.starknet.write().await;

    let is_dry_run = if let Some(data) = data {
        let data = Json(data);
        data.dry_run
    } else {
        false
    };

    // Fetch and execute messages to l2.
    let (messages_to_l2, generated_l2_transactions) = if is_dry_run {
        (vec![], vec![])
    } else {
        let messages = starknet.fetch_messages_to_l2().await.map_err(|e| {
            HttpApiError::MessagingError { msg: format!("fetch messages to l2: {}", e) }
        })?;

        let tx_hashes = starknet.execute_messages_to_l2(&messages).await.map_err(|e| {
            HttpApiError::MessagingError { msg: format!("execute messages to l2: {}", e) }
        })?;

        (messages, tx_hashes)
    };

    // Collect and send messages to L1.
    let messages_to_l1 = starknet.collect_messages_to_l1().await.map_err(|e| {
        HttpApiError::MessagingError { msg: format!("collect messages to l1 error: {}", e) }
    })?;

    if is_dry_run {
        return Ok(Json(FlushedMessages {
            messages_to_l1,
            messages_to_l2,
            generated_l2_transactions,
            l1_provider: "dry run".to_string(),
        }));
    }

    starknet.send_messages_to_l1().await.map_err(|e| HttpApiError::MessagingError {
        msg: format!("send messages to l1 error: {}", e),
    })?;

    let l1_provider = starknet.get_ethereum_url().unwrap_or("Not set".to_string());

    Ok(Json(FlushedMessages {
        messages_to_l1,
        messages_to_l2,
        generated_l2_transactions,
        l1_provider,
    }))
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
    let mut starknet = state.api.starknet.write().await;

    let message_hash = starknet
        .consume_l2_to_l1_message(&message)
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(Json(MessageHash { message_hash }))
}
