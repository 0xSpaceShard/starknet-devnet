use starknet_types::felt::{TransactionHash, felt_from_prefixed_hex};
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;

use crate::api::error::{ApiError, StrictRpcResult};
use crate::api::models::{
    DevnetResponse, FlushParameters, FlushedMessages, MessageHash, MessagingLoadAddress,
    PostmanLoadL1MessagingContract, TransactionHashOutput,
};
use crate::api::{Api, JsonRpcHandler};
use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::ResponseResult;
use crate::rpc_handler::RpcHandler;

pub(crate) async fn postman_load_impl(
    api: &Api,
    data: PostmanLoadL1MessagingContract,
) -> StrictRpcResult {
    let mut starknet = api.starknet.lock().await;
    let messaging_contract_address = starknet
        .configure_messaging(
            &data.network_url,
            data.messaging_contract_address.as_deref(),
            data.deployer_account_private_key.as_deref(),
        )
        .await?;

    Ok(DevnetResponse::MessagingContractAddress(MessagingLoadAddress {
        messaging_contract_address,
    })
    .into())
}

async fn execute_rpc_tx(
    rpc_handler: &JsonRpcHandler,
    rpc_call: RpcMethodCall,
) -> Result<TransactionHash, RpcError> {
    match rpc_handler.on_call(rpc_call).await.result {
        ResponseResult::Success(result) => {
            let tx_hash_hex = result
                .get("transaction_hash")
                .ok_or(RpcError::internal_error_with(format!(
                    "Message execution did not yield a transaction hash: {result:?}"
                )))?
                .as_str()
                .ok_or(RpcError::internal_error_with(format!(
                    "Message execution result contains invalid transaction hash: {result:?}"
                )))?;
            let tx_hash = felt_from_prefixed_hex(tx_hash_hex).map_err(|e| {
                RpcError::internal_error_with(format!(
                    "Message execution resulted in an invalid tx hash: {tx_hash_hex}: {e}"
                ))
            })?;
            Ok(tx_hash)
        }
        ResponseResult::Error(e) => Err(e),
    }
}

pub(crate) async fn postman_flush_impl(
    api: &Api,
    data: Option<FlushParameters>,
    rpc_handler: &JsonRpcHandler,
) -> StrictRpcResult {
    let is_dry_run = if let Some(params) = data { params.dry_run } else { false };

    // Need to handle L1 to L2 first in case those messages create L2 to L1 messages.
    let mut messages_to_l2 = vec![];
    let mut generated_l2_transactions = vec![];
    if !is_dry_run {
        // Fetch and execute messages to L2.
        // It is important that api.starknet is dropped immediately to allow rpc execution
        messages_to_l2 = api.starknet.lock().await.fetch_messages_to_l2().await.map_err(|e| {
            ApiError::RpcError(RpcError::internal_error_with(format!(
                "Error in fetching messages to L2: {e}"
            )))
        })?;

        for message in &messages_to_l2 {
            let rpc_call = message.try_into().map_err(|e| {
                ApiError::RpcError(RpcError::internal_error_with(format!(
                    "Error in converting message to L2 RPC call: {e}"
                )))
            })?;
            let tx_hash =
                execute_rpc_tx(rpc_handler, rpc_call).await.map_err(ApiError::RpcError)?;
            generated_l2_transactions.push(tx_hash);
        }
    };

    // Collect and send messages to L1.
    let mut starknet = api.starknet.lock().await;
    let messages_to_l1 = starknet.collect_messages_to_l1().await.map_err(|e| {
        ApiError::RpcError(RpcError::internal_error_with(format!(
            "Error in collecting messages to L1: {e}"
        )))
    })?;

    let l1_provider = if is_dry_run {
        "dry run".to_string()
    } else {
        starknet.send_messages_to_l1().await.map_err(|e| {
            ApiError::RpcError(RpcError::internal_error_with(format!(
                "Error in sending messages to L1: {e}"
            )))
        })?;
        starknet.get_ethereum_url().unwrap_or("Not set".to_string())
    };

    let flushed_messages =
        FlushedMessages { messages_to_l1, messages_to_l2, generated_l2_transactions, l1_provider };

    Ok(DevnetResponse::FlushedMessages(flushed_messages).into())
}

pub async fn postman_send_message_to_l2_impl(api: &Api, message: MessageToL2) -> StrictRpcResult {
    let transaction = L1HandlerTransaction::try_from_message_to_l2(message)?;
    let transaction_hash = api.starknet.lock().await.add_l1_handler_transaction(transaction)?;
    Ok(DevnetResponse::TransactionHash(TransactionHashOutput { transaction_hash }).into())
}

pub async fn postman_consume_message_from_l2_impl(
    api: &Api,
    message: MessageToL1,
) -> StrictRpcResult {
    let message_hash = api.starknet.lock().await.consume_l2_to_l1_message(&message).await?;
    Ok(DevnetResponse::MessageHash(MessageHash { message_hash }).into())
}
