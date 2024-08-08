use axum::extract::State;
use axum::Json;
use starknet_rs_core::types::Felt;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{
    FlushParameters, FlushedMessages, MessageHash, MessagingLoadAddress,
    PostmanLoadL1MessagingContract, TxHash,
};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::Api;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::ResponseResult;
use crate::rpc_handler::RpcHandler;

pub async fn postman_load(
    State(state): State<HttpApiHandler>,
    Json(data): Json<PostmanLoadL1MessagingContract>,
) -> HttpApiResult<Json<MessagingLoadAddress>> {
    postman_load_impl(&state.api, data).await.map(Json::from)
}

pub async fn postman_flush(
    State(_state): State<HttpApiHandler>,
    _optional_data: Option<Json<FlushParameters>>,
) -> HttpApiResult<Json<FlushedMessages>> {
    todo!("Should never be called");
    // postman_flush_impl(&state.api, extract_optional_json_from_request(optional_data))
    //     .await
    //     .map(Json::from)
}

pub async fn postman_send_message_to_l2(
    State(_state): State<HttpApiHandler>,
    Json(_message): Json<MessageToL2>,
) -> HttpApiResult<Json<TxHash>> {
    todo!("Should never be called");
    // postman_send_message_to_l2_impl(&state.api, message).await.map(Json::from)
}

pub async fn postman_consume_message_from_l2(
    State(state): State<HttpApiHandler>,
    Json(message): Json<MessageToL1>,
) -> HttpApiResult<Json<MessageHash>> {
    postman_consume_message_from_l2_impl(&state.api, message).await.map(Json::from)
}

pub(crate) async fn postman_load_impl(
    api: &Api,
    data: PostmanLoadL1MessagingContract,
) -> HttpApiResult<MessagingLoadAddress> {
    let mut starknet = api.starknet.lock().await;

    let messaging_contract_address = starknet
        .configure_messaging(&data.network_url, data.address.as_deref())
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(MessagingLoadAddress { messaging_contract_address })
}

async fn execute_rpc_tx(
    rpc_handler: &JsonRpcHandler,
    rpc_call: RpcMethodCall,
) -> Result<TransactionHash, HttpApiError> {
    match rpc_handler.on_call(rpc_call).await.result {
        ResponseResult::Success(result) => {
            let tx_hash_hex = result
                .get("transaction_hash")
                .ok_or(HttpApiError::MessagingError {
                    msg: format!("Message execution did not yield a transaction hash: {result:?}"),
                })?
                .as_str()
                .ok_or(HttpApiError::MessagingError {
                    msg: format!("Result contains invalid transaction hash: {result:?}"),
                })?;
            let tx_hash = Felt::from_hex(tx_hash_hex).map_err(|e| {
                HttpApiError::MessagingError { msg: format!("Invalid tx hash: {tx_hash_hex}: {e}") }
            })?;
            Ok(tx_hash)
        }
        ResponseResult::Error(e) => {
            Err(HttpApiError::MessagingError { msg: format!("Transaction execution error: {e}") })
        }
    }
}

pub(crate) async fn postman_flush_impl(
    api: &Api,
    data: Option<FlushParameters>,
    rpc_handler: &JsonRpcHandler,
) -> HttpApiResult<FlushedMessages> {
    // Need to handle L1 to L2 first in case those messages create L2 to L1 messages.
    let mut starknet = api.starknet.lock().await;

    let is_dry_run = if let Some(params) = data { params.dry_run } else { false };

    let mut messages_to_l2 = vec![];
    let mut generated_l2_transactions = vec![];
    if !is_dry_run {
        // Fetch and execute messages to L2.
        messages_to_l2 = starknet.fetch_messages_to_l2().await.map_err(|e| {
            HttpApiError::MessagingError { msg: format!("Error in fetching messages to l2: {e}") }
        })?;

        drop(starknet); // drop to avoid deadlock, later re-acquire

        for message in &messages_to_l2 {
            let rpc_call = message.try_into().map_err(|e: crate::error::Error| {
                HttpApiError::MessagingError { msg: e.to_string() }
            })?;
            let tx_hash = execute_rpc_tx(rpc_handler, rpc_call).await?;
            generated_l2_transactions.push(tx_hash);
        }

        starknet = api.starknet.lock().await;
    };

    // Collect and send messages to L1.
    let messages_to_l1 = starknet.collect_messages_to_l1().await.map_err(|e| {
        HttpApiError::MessagingError { msg: format!("collect messages to l1 error: {}", e) }
    })?;

    if is_dry_run {
        return Ok(FlushedMessages {
            messages_to_l1,
            messages_to_l2,
            generated_l2_transactions,
            l1_provider: "dry run".to_string(),
        });
    }

    starknet.send_messages_to_l1().await.map_err(|e| HttpApiError::MessagingError {
        msg: format!("send messages to l1 error: {}", e),
    })?;

    let l1_provider = starknet.get_ethereum_url().unwrap_or("Not set".to_string());

    Ok(FlushedMessages { messages_to_l1, messages_to_l2, generated_l2_transactions, l1_provider })
}

pub async fn postman_send_message_to_l2_impl(
    api: &Api,
    message: MessageToL2,
) -> HttpApiResult<TxHash> {
    let mut starknet = api.starknet.lock().await;

    let transaction = L1HandlerTransaction::try_from_message_to_l2(message)
        .map_err(|e| HttpApiError::InvalidValueError { msg: e.to_string() })?;

    let transaction_hash = starknet
        .add_l1_handler_transaction(transaction)
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(TxHash { transaction_hash })
}

pub async fn postman_consume_message_from_l2_impl(
    api: &Api,
    message: MessageToL1,
) -> HttpApiResult<MessageHash> {
    let mut starknet = api.starknet.lock().await;

    let message_hash = starknet
        .consume_l2_to_l1_message(&message)
        .await
        .map_err(|e| HttpApiError::MessagingError { msg: e.to_string() })?;

    Ok(MessageHash { message_hash })
}
