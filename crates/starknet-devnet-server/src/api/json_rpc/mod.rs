mod endpoints;
mod endpoints_ws;
pub mod error;
pub mod models;
pub(crate) mod origin_forwarder;
#[cfg(test)]
mod spec_reader;
mod write_endpoints;

pub const RPC_SPEC_VERSION: &str = "0.8.0-rc3";

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use enum_helper_macros::{AllVariantsSerdeRenames, VariantName};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use models::{
    BlockAndClassHashInput, BlockAndContractAddressInput, BlockAndIndexInput, CallInput,
    ClassHashInput, EstimateFeeInput, EventsInput, EventsSubscriptionInput, GetStorageInput,
    GetStorageProofInput, L1TransactionHashInput, PendingTransactionsSubscriptionInput,
    SubscriptionBlockIdInput, SubscriptionIdInput, TransactionHashInput, TransactionHashOutput,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet_core::starknet::starknet_config::{DumpOn, StarknetConfig};
use starknet_core::{CasmContractClass, StarknetBlock};
use starknet_rs_core::types::{BlockId, BlockTag, ContractClass as CodegenContractClass, Felt};
use starknet_types::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::block::{Block, PendingBlock, ReorgData};
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::gas_modification::{GasModification, GasModificationRequest};
use starknet_types::rpc::state::{PendingStateUpdate, StateUpdate};
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::{
    BlockTransactionTrace, EventsChunk, L1HandlerTransactionStatus, SimulatedTransaction,
    TransactionStatus, TransactionTrace, TransactionWithHash,
};
use starknet_types::starknet_api::block::BlockNumber;
use tokio::sync::Mutex;
use tracing::{error, info, trace};

use self::error::StrictRpcResult;
use self::models::{
    AccountAddressInput, BlockHashAndNumberOutput, BlockIdInput,
    BroadcastedDeclareTransactionInput, BroadcastedDeployAccountTransactionInput,
    BroadcastedInvokeTransactionInput, DeclareTransactionOutput, DeployAccountTransactionOutput,
    SyncingOutput,
};
use self::origin_forwarder::OriginForwarder;
use super::http::endpoints::accounts::{BalanceQuery, PredeployedAccountsQuery};
use super::http::endpoints::DevnetConfig;
use super::http::models::{
    AbortedBlocks, AbortingBlocks, AccountBalanceResponse, CreatedBlock, DumpPath,
    DumpResponseBody, FlushParameters, FlushedMessages, IncreaseTime, IncreaseTimeResponse,
    LoadPath, MessageHash, MessagingLoadAddress, MintTokensRequest, MintTokensResponse,
    PostmanLoadL1MessagingContract, RestartParameters, SerializableAccount, SetTime,
    SetTimeResponse,
};
use super::Api;
use crate::api::json_rpc::models::{
    BroadcastedDeclareTransactionEnumWrapper, BroadcastedDeployAccountTransactionEnumWrapper,
    BroadcastedInvokeTransactionEnumWrapper, SimulateTransactionsInput,
};
use crate::api::serde_helpers::{empty_params, optional_params};
use crate::dump_util::dump_event;
use crate::restrictive_mode::is_json_rpc_method_restricted;
use crate::rpc_core::error::{ErrorCode, RpcError};
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::{ResponseResult, RpcResponse};
use crate::rpc_handler::RpcHandler;
use crate::subscribe::{
    NewTransactionStatus, NotificationData, PendingTransactionNotification, SocketId,
    TransactionHashWrapper,
};
use crate::ServerConfig;

/// Helper trait to easily convert results to rpc results
pub trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

/// Used when there is no defined code to use
pub const WILDCARD_RPC_ERROR_CODE: i64 = -1;

/// Converts a serializable value into a `ResponseResult`
pub fn to_rpc_result<T: Serialize>(val: T) -> ResponseResult {
    match serde_json::to_value(val) {
        Ok(success) => ResponseResult::Success(success),
        Err(err) => {
            error!("Failed serialize rpc response: {:?}", err);
            ResponseResult::error(RpcError::internal_error())
        }
    }
}

impl ToRpcResponseResult for StrictRpcResult {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(JsonRpcResponse::Empty) => to_rpc_result(json!({})),
            Ok(JsonRpcResponse::Devnet(data)) => to_rpc_result(data),
            Ok(JsonRpcResponse::Starknet(data)) => to_rpc_result(data),
            Err(err) => err.api_error_to_rpc_error().into(),
        }
    }
}

/// This object will be used as a shared state between HTTP calls.
/// Is similar to the HttpApiHandler but is with extended functionality and is used for JSON-RPC
/// methods
#[derive(Clone)]
pub struct JsonRpcHandler {
    pub api: Api,
    pub origin_caller: Option<OriginForwarder>,
    pub starknet_config: StarknetConfig,
    pub server_config: ServerConfig,
}

fn log_if_deprecated_tx(request: &JsonRpcRequest) {
    let is_deprecated_tx = match request {
        JsonRpcRequest::AddDeclareTransaction(BroadcastedDeclareTransactionInput {
            declare_transaction: BroadcastedDeclareTransactionEnumWrapper::Declare(tx),
        }) => tx.is_deprecated(),
        JsonRpcRequest::AddDeployAccountTransaction(BroadcastedDeployAccountTransactionInput {
            deploy_account_transaction:
                BroadcastedDeployAccountTransactionEnumWrapper::DeployAccount(tx),
        }) => tx.is_deprecated(),
        JsonRpcRequest::AddInvokeTransaction(BroadcastedInvokeTransactionInput {
            invoke_transaction: BroadcastedInvokeTransactionEnumWrapper::Invoke(tx),
        }) => tx.is_deprecated(),
        JsonRpcRequest::EstimateFee(EstimateFeeInput { request: txs, .. }) => {
            txs.iter().any(|tx| tx.is_deprecated())
        }
        JsonRpcRequest::SimulateTransactions(SimulateTransactionsInput {
            transactions, ..
        }) => transactions.iter().any(|tx| tx.is_deprecated()),
        _ => false,
    };

    if is_deprecated_tx {
        tracing::warn!(
            "Received a transaction of a deprecated version! Please modify or upgrade your \
             Starknet client to use v3 transactions."
        );
    }
}

#[async_trait::async_trait]
impl RpcHandler for JsonRpcHandler {
    type Request = JsonRpcRequest;

    async fn on_request(
        &self,
        request: Self::Request,
        original_call: RpcMethodCall,
    ) -> ResponseResult {
        info!(target: "rpc", "received method in on_request {}", request);
        log_if_deprecated_tx(&request);

        let is_request_forwardable = request.is_forwardable_to_origin(); // applicable if forking
        let is_request_dumpable = request.is_dumpable();

        // for later comparison and subscription notifications
        let old_latest_block = if request.requires_notifying() {
            Some(self.get_block_by_tag(BlockTag::Latest).await)
        } else {
            None
        };

        let old_pending_block =
            if request.requires_notifying() && self.starknet_config.uses_pending_block() {
                Some(self.get_block_by_tag(BlockTag::Pending).await)
            } else {
                None
            };

        let starknet_resp = self.execute(request).await;

        // If locally we got an error and forking is set up, forward the request to the origin
        if let (Err(err), Some(forwarder)) = (&starknet_resp, &self.origin_caller) {
            if err.is_forwardable_to_origin() && is_request_forwardable {
                // if a block or state is requested that was only added to origin after
                // forking happened, it will be normally returned; we don't extra-handle this case
                return forwarder.call(&original_call).await;
            }
        }

        if starknet_resp.is_ok() && is_request_dumpable {
            if let Err(e) = self.update_dump(&original_call).await {
                return ResponseResult::Error(e);
            }
        }

        if let Err(e) = self.broadcast_changes(old_latest_block, old_pending_block).await {
            return ResponseResult::Error(e.api_error_to_rpc_error());
        }

        starknet_resp.to_rpc_result()
    }

    async fn on_call(&self, call: RpcMethodCall) -> RpcResponse {
        trace!(target: "rpc",  id = ?call.id , method = ?call.method, "received method call");

        if !self.allows_method(&call.method) {
            return RpcResponse::from_rpc_error(RpcError::new(ErrorCode::MethodForbidden), call.id);
        }

        match to_json_rpc_request(&call) {
            Ok(req) => {
                let result = self.on_request(req, call.clone()).await;
                RpcResponse::new(call.id, result)
            }
            Err(e) => RpcResponse::from_rpc_error(e, call.id),
        }
    }

    async fn on_websocket(&self, socket: WebSocket) {
        let (socket_writer, mut socket_reader) = socket.split();
        let socket_writer = Arc::new(Mutex::new(socket_writer));

        let socket_id = self.api.sockets.lock().await.insert(socket_writer.clone());

        // listen to new messages coming through the socket
        let mut socket_safely_closed = false;
        while let Some(msg) = socket_reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.on_websocket_call(text.as_bytes(), socket_writer.clone(), socket_id).await;
                }
                Ok(Message::Binary(bytes)) => {
                    self.on_websocket_call(&bytes, socket_writer.clone(), socket_id).await;
                }
                Ok(Message::Close(_)) => {
                    socket_safely_closed = true;
                    break;
                }
                other => {
                    tracing::error!("Socket handler got an unexpected message: {other:?}")
                }
            }
        }

        if socket_safely_closed {
            self.api.sockets.lock().await.remove(&socket_id);
            tracing::info!("Websocket disconnected");
        } else {
            tracing::error!("Failed socket read");
        }
    }
}

impl JsonRpcHandler {
    pub fn new(
        api: Api,
        starknet_config: &StarknetConfig,
        server_config: &ServerConfig,
    ) -> JsonRpcHandler {
        let origin_caller = if let (Some(url), Some(block_number)) =
            (&starknet_config.fork_config.url, starknet_config.fork_config.block_number)
        {
            Some(OriginForwarder::new(url.to_string(), block_number))
        } else {
            None
        };

        JsonRpcHandler {
            api,
            origin_caller,
            starknet_config: starknet_config.clone(),
            server_config: server_config.clone(),
        }
    }

    /// The latest and pending block are always defined, so to avoid having to deal with Err/None in
    /// places where this method is called, it is defined to return an empty accepted block,
    /// even though that case should never happen.
    async fn get_block_by_tag(&self, tag: BlockTag) -> StarknetBlock {
        let starknet = self.api.starknet.lock().await;
        match starknet.get_block(&BlockId::Tag(tag)) {
            Ok(block) => block.clone(),
            _ => StarknetBlock::create_empty_accepted(),
        }
    }

    async fn broadcast_pending_tx_changes(
        &self,
        old_pending_block: StarknetBlock,
    ) -> Result<(), error::ApiError> {
        let new_pending_block = self.get_block_by_tag(BlockTag::Pending).await;
        let old_pending_txs = old_pending_block.get_transactions();
        let new_pending_txs = new_pending_block.get_transactions();

        if new_pending_txs.len() > old_pending_txs.len() {
            #[allow(clippy::expect_used)]
            let new_tx_hash = new_pending_txs.last().expect("has at least one element");

            let starknet = self.api.starknet.lock().await;

            let mut notifications = vec![];

            let status = starknet
                .get_transaction_execution_and_finality_status(*new_tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;

            notifications.push(NotificationData::TransactionStatus(NewTransactionStatus {
                transaction_hash: *new_tx_hash,
                status,
            }));

            let tx = starknet
                .get_transaction_by_hash(*new_tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;

            notifications.push(NotificationData::PendingTransaction(
                PendingTransactionNotification::Full(Box::new(tx.clone())),
            ));

            notifications.push(NotificationData::PendingTransaction(
                PendingTransactionNotification::Hash(TransactionHashWrapper {
                    hash: *tx.get_transaction_hash(),
                    sender_address: tx.get_sender_address(),
                }),
            ));

            let events = starknet.get_unlimited_events(
                Some(BlockId::Tag(BlockTag::Pending)),
                Some(BlockId::Tag(BlockTag::Pending)),
                None,
                None,
            )?;
            for event in events.into_iter().filter(|e| &e.transaction_hash == new_tx_hash) {
                notifications.push(NotificationData::Event(event));
            }

            self.api.sockets.lock().await.notify_subscribers(&notifications).await;
        }

        Ok(())
    }

    async fn broadcast_latest_changes(
        &self,
        new_latest_block: StarknetBlock,
    ) -> Result<(), error::ApiError> {
        let block_header = (&new_latest_block).into();
        let mut notifications = vec![NotificationData::NewHeads(block_header)];

        let starknet = self.api.starknet.lock().await;

        for tx_hash in new_latest_block.get_transactions() {
            if !self.starknet_config.uses_pending_block() {
                let tx = starknet
                    .get_transaction_by_hash(*tx_hash)
                    .map_err(error::ApiError::StarknetDevnetError)?;

                // There are no pending txs in this mode, but basically we are pretending that the
                // transaction existed for a short period of time in the pending block, thus
                // triggering the notification. This is important for users depending on this
                // subscription type to find out about all new transactions.
                notifications.push(NotificationData::PendingTransaction(
                    PendingTransactionNotification::Full(Box::new(tx.clone())),
                ));
                notifications.push(NotificationData::PendingTransaction(
                    PendingTransactionNotification::Hash(TransactionHashWrapper {
                        hash: *tx_hash,
                        sender_address: tx.get_sender_address(),
                    }),
                ));

                // If pending block used, tx status notifications have already been sent.
                // If we are here, pending block is not used and subscribers need to be notified.
                let status = starknet
                    .get_transaction_execution_and_finality_status(*tx_hash)
                    .map_err(error::ApiError::StarknetDevnetError)?;
                notifications.push(NotificationData::TransactionStatus(NewTransactionStatus {
                    transaction_hash: *tx_hash,
                    status,
                }));

                let events = starknet.get_unlimited_events(
                    Some(BlockId::Tag(BlockTag::Latest)),
                    Some(BlockId::Tag(BlockTag::Latest)),
                    None,
                    None,
                )?;
                for event in events {
                    notifications.push(NotificationData::Event(event));
                }
            }
        }

        self.api.sockets.lock().await.notify_subscribers(&notifications).await;
        Ok(())
    }

    /// Notify subscribers of what they are subscribed to.
    async fn broadcast_changes(
        &self,
        old_latest_block: Option<StarknetBlock>,
        old_pending_block: Option<StarknetBlock>,
    ) -> Result<(), error::ApiError> {
        let old_latest_block = if let Some(block) = old_latest_block {
            block
        } else {
            return Ok(());
        };

        if let Some(old_pending_block) = old_pending_block {
            self.broadcast_pending_tx_changes(old_pending_block).await?;
        }

        let new_latest_block = self.get_block_by_tag(BlockTag::Latest).await;

        match new_latest_block.block_number().cmp(&old_latest_block.block_number()) {
            std::cmp::Ordering::Less => {
                self.broadcast_reorg(old_latest_block, new_latest_block).await?
            }
            std::cmp::Ordering::Equal => { /* no changes required */ }
            std::cmp::Ordering::Greater => self.broadcast_latest_changes(new_latest_block).await?,
        }

        Ok(())
    }

    async fn broadcast_reorg(
        &self,
        old_latest_block: StarknetBlock,
        new_latest_block: StarknetBlock,
    ) -> Result<(), error::ApiError> {
        // Since it is impossible to determine the hash of the former successor of new_latest_block
        // directly, we iterate from old_latest_block all the way to the aborted successor of
        // new_latest_block.
        let new_latest_hash = new_latest_block.block_hash();
        let mut orphan_starting_block_hash = old_latest_block.block_hash();
        let starknet = self.api.starknet.lock().await;
        loop {
            let orphan_block = starknet.get_block(&BlockId::Hash(orphan_starting_block_hash))?;
            let parent_hash = orphan_block.parent_hash();
            if parent_hash == new_latest_hash {
                break;
            }
            orphan_starting_block_hash = parent_hash;
        }

        let notification = NotificationData::Reorg(ReorgData {
            starting_block_hash: orphan_starting_block_hash,
            starting_block_number: new_latest_block.block_number().unchecked_next(),
            ending_block_hash: old_latest_block.block_hash(),
            ending_block_number: old_latest_block.block_number(),
        });

        self.api.sockets.lock().await.notify_subscribers(&[notification]).await;
        Ok(())
    }

    /// Matches the request to the corresponding enum variant and executes the request.
    async fn execute(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, error::ApiError> {
        trace!(target: "JsonRpcHandler::execute", "executing starknet request");

        match request {
            JsonRpcRequest::SpecVersion => self.spec_version(),
            JsonRpcRequest::BlockWithTransactionHashes(block) => {
                self.get_block_with_tx_hashes(block.block_id).await
            }
            JsonRpcRequest::BlockWithFullTransactions(block) => {
                self.get_block_with_txs(block.block_id).await
            }
            JsonRpcRequest::BlockWithReceipts(block) => {
                self.get_block_with_receipts(block.block_id).await
            }
            JsonRpcRequest::StateUpdate(block) => self.get_state_update(block.block_id).await,
            JsonRpcRequest::StorageAt(GetStorageInput { contract_address, key, block_id }) => {
                self.get_storage_at(contract_address, key, block_id).await
            }
            JsonRpcRequest::TransactionStatusByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_status_by_hash(transaction_hash).await
            }
            JsonRpcRequest::TransactionByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_by_hash(transaction_hash).await
            }
            JsonRpcRequest::TransactionByBlockAndIndex(BlockAndIndexInput { block_id, index }) => {
                self.get_transaction_by_block_id_and_index(block_id, index).await
            }
            JsonRpcRequest::TransactionReceiptByTransactionHash(TransactionHashInput {
                transaction_hash,
            }) => self.get_transaction_receipt_by_hash(transaction_hash).await,
            JsonRpcRequest::ClassByHash(BlockAndClassHashInput { block_id, class_hash }) => {
                self.get_class(block_id, class_hash).await
            }
            JsonRpcRequest::CompiledCasmByClassHash(ClassHashInput { class_hash }) => {
                self.get_compiled_casm(class_hash).await
            }
            JsonRpcRequest::ClassHashAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_hash_at(block_id, contract_address).await,
            JsonRpcRequest::ClassAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_at(block_id, contract_address).await,
            JsonRpcRequest::BlockTransactionCount(block) => {
                self.get_block_txs_count(block.block_id).await
            }
            JsonRpcRequest::Call(CallInput { request, block_id }) => {
                self.call(block_id, request).await
            }
            JsonRpcRequest::EstimateFee(EstimateFeeInput {
                request,
                block_id,
                simulation_flags,
            }) => self.estimate_fee(block_id, request, simulation_flags).await,
            JsonRpcRequest::BlockNumber => self.block_number().await,
            JsonRpcRequest::BlockHashAndNumber => self.block_hash_and_number().await,
            JsonRpcRequest::ChainId => self.chain_id().await,
            JsonRpcRequest::Syncing => self.syncing().await,
            JsonRpcRequest::Events(EventsInput { filter }) => self.get_events(filter).await,
            JsonRpcRequest::ContractNonce(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_nonce(block_id, contract_address).await,
            JsonRpcRequest::AddDeclareTransaction(BroadcastedDeclareTransactionInput {
                declare_transaction,
            }) => {
                let BroadcastedDeclareTransactionEnumWrapper::Declare(broadcasted_transaction) =
                    declare_transaction;
                self.add_declare_transaction(broadcasted_transaction).await
            }
            JsonRpcRequest::AddDeployAccountTransaction(
                BroadcastedDeployAccountTransactionInput { deploy_account_transaction },
            ) => {
                let BroadcastedDeployAccountTransactionEnumWrapper::DeployAccount(
                    broadcasted_transaction,
                ) = deploy_account_transaction;
                self.add_deploy_account_transaction(broadcasted_transaction).await
            }
            JsonRpcRequest::AddInvokeTransaction(BroadcastedInvokeTransactionInput {
                invoke_transaction,
            }) => {
                let BroadcastedInvokeTransactionEnumWrapper::Invoke(broadcasted_transaction) =
                    invoke_transaction;
                self.add_invoke_transaction(broadcasted_transaction).await
            }
            JsonRpcRequest::EstimateMessageFee(request) => {
                self.estimate_message_fee(request.get_block_id(), request.get_raw_message().clone())
                    .await
            }
            JsonRpcRequest::SimulateTransactions(SimulateTransactionsInput {
                block_id,
                transactions,
                simulation_flags,
            }) => self.simulate_transactions(block_id, transactions, simulation_flags).await,
            JsonRpcRequest::TraceTransaction(TransactionHashInput { transaction_hash }) => {
                self.get_trace_transaction(transaction_hash).await
            }
            JsonRpcRequest::BlockTransactionTraces(BlockIdInput { block_id }) => {
                self.get_trace_block_transactions(block_id).await
            }
            JsonRpcRequest::ImpersonateAccount(AccountAddressInput { account_address }) => {
                self.impersonate_account(account_address).await
            }
            JsonRpcRequest::StopImpersonateAccount(AccountAddressInput { account_address }) => {
                self.stop_impersonating_account(account_address).await
            }
            JsonRpcRequest::AutoImpersonate => self.set_auto_impersonate(true).await,
            JsonRpcRequest::StopAutoImpersonate => self.set_auto_impersonate(false).await,
            JsonRpcRequest::Dump(path) => self.dump(path).await,
            JsonRpcRequest::Load(LoadPath { path }) => self.load(path).await,
            JsonRpcRequest::PostmanLoadL1MessagingContract(data) => self.postman_load(data).await,
            JsonRpcRequest::PostmanFlush(data) => self.postman_flush(data).await,
            JsonRpcRequest::PostmanSendMessageToL2(message) => {
                self.postman_send_message_to_l2(message).await
            }
            JsonRpcRequest::PostmanConsumeMessageFromL2(message) => {
                self.postman_consume_message_from_l2(message).await
            }
            JsonRpcRequest::CreateBlock => self.create_block().await,
            JsonRpcRequest::AbortBlocks(data) => self.abort_blocks(data).await,
            JsonRpcRequest::SetGasPrice(data) => self.set_gas_price(data).await,
            JsonRpcRequest::Restart(data) => self.restart(data).await,
            JsonRpcRequest::SetTime(data) => self.set_time(data).await,
            JsonRpcRequest::IncreaseTime(data) => self.increase_time(data).await,
            JsonRpcRequest::PredeployedAccounts(data) => self.get_predeployed_accounts(data).await,
            JsonRpcRequest::AccountBalance(data) => self.get_account_balance(data).await,
            JsonRpcRequest::Mint(data) => self.mint(data).await,
            JsonRpcRequest::DevnetConfig => self.get_devnet_config().await,
            JsonRpcRequest::MessagesStatusByL1Hash(data) => self.get_messages_status(data).await,
            JsonRpcRequest::StorageProof(data) => self.get_storage_proof(data).await,
        }
    }

    /// Takes `bytes` to be an encoded RPC call, executes it, and sends the response back via `ws`.
    async fn on_websocket_call(
        &self,
        bytes: &[u8],
        ws: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        socket_id: SocketId,
    ) {
        let error_serialized = match serde_json::from_slice(bytes) {
            Ok(rpc_call) => match self.on_websocket_rpc_call(&rpc_call, socket_id).await {
                Ok(_) => return,
                Err(e) => json!(RpcResponse::from_rpc_error(e, rpc_call.id)).to_string(),
            },
            Err(e) => e.to_string(),
        };

        if let Err(e) = ws.lock().await.send(Message::Text(error_serialized)).await {
            tracing::error!("Error sending websocket message: {e}");
        }
    }

    fn allows_method(&self, method: &String) -> bool {
        if let Some(restricted_methods) = &self.server_config.restricted_methods {
            if is_json_rpc_method_restricted(method, restricted_methods) {
                return false;
            }
        }

        true
    }

    /// Since some subscriptions might need to send multiple messages, sending messages other than
    /// errors is left to individual RPC method handlers and this method returns an empty successful
    /// Result.
    async fn on_websocket_rpc_call(
        &self,
        call: &RpcMethodCall,
        socket_id: SocketId,
    ) -> Result<(), RpcError> {
        trace!(target: "rpc",  id = ?call.id , method = ?call.method, "received websocket call");

        let req = to_json_rpc_request(call)?;
        self.execute_ws(req, call.id.clone(), socket_id)
            .await
            .map_err(|e| e.api_error_to_rpc_error())
    }

    async fn update_dump(&self, event: &RpcMethodCall) -> Result<(), RpcError> {
        match self.starknet_config.dump_on {
            Some(DumpOn::Block) => {
                let dump_path = self
                    .starknet_config
                    .dump_path
                    .as_deref()
                    .ok_or(RpcError::internal_error_with("Undefined dump_path"))?;

                dump_event(event, dump_path).map_err(|e| {
                    let msg = format!("Failed dumping of {}: {e}", event.method);
                    RpcError::internal_error_with(msg)
                })?;
            }
            Some(DumpOn::Request | DumpOn::Exit) => {
                self.api.dumpable_events.lock().await.push(event.clone())
            }
            None => (),
        }

        Ok(())
    }

    pub async fn re_execute(&self, events: &[RpcMethodCall]) -> Result<(), RpcError> {
        for event in events {
            if let ResponseResult::Error(e) = self.on_call(event.clone()).await.result {
                return Err(e);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, AllVariantsSerdeRenames, VariantName)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "method", content = "params")]
pub enum JsonRpcRequest {
    #[serde(rename = "starknet_specVersion", with = "empty_params")]
    SpecVersion,
    #[serde(rename = "starknet_getBlockWithTxHashes")]
    BlockWithTransactionHashes(BlockIdInput),
    #[serde(rename = "starknet_getBlockWithTxs")]
    BlockWithFullTransactions(BlockIdInput),
    #[serde(rename = "starknet_getBlockWithReceipts")]
    BlockWithReceipts(BlockIdInput),
    #[serde(rename = "starknet_getStateUpdate")]
    StateUpdate(BlockIdInput),
    #[serde(rename = "starknet_getStorageAt")]
    StorageAt(GetStorageInput),
    #[serde(rename = "starknet_getStorageProof")]
    StorageProof(GetStorageProofInput),
    #[serde(rename = "starknet_getTransactionByHash")]
    TransactionByHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionByBlockIdAndIndex")]
    TransactionByBlockAndIndex(BlockAndIndexInput),
    #[serde(rename = "starknet_getTransactionReceipt")]
    TransactionReceiptByTransactionHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionStatus")]
    TransactionStatusByHash(TransactionHashInput),
    #[serde(rename = "starknet_getMessagesStatus")]
    MessagesStatusByL1Hash(L1TransactionHashInput),
    #[serde(rename = "starknet_getClass")]
    ClassByHash(BlockAndClassHashInput),
    #[serde(rename = "starknet_getCompiledCasm")]
    CompiledCasmByClassHash(ClassHashInput),
    #[serde(rename = "starknet_getClassHashAt")]
    ClassHashAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getClassAt")]
    ClassAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getBlockTransactionCount")]
    BlockTransactionCount(BlockIdInput),
    #[serde(rename = "starknet_call")]
    Call(CallInput),
    #[serde(rename = "starknet_estimateFee")]
    EstimateFee(EstimateFeeInput),
    #[serde(rename = "starknet_blockNumber", with = "empty_params")]
    BlockNumber,
    #[serde(rename = "starknet_blockHashAndNumber", with = "empty_params")]
    BlockHashAndNumber,
    #[serde(rename = "starknet_chainId", with = "empty_params")]
    ChainId,
    #[serde(rename = "starknet_syncing", with = "empty_params")]
    Syncing,
    #[serde(rename = "starknet_getEvents")]
    Events(EventsInput),
    #[serde(rename = "starknet_getNonce")]
    ContractNonce(BlockAndContractAddressInput),
    #[serde(rename = "starknet_addDeclareTransaction")]
    AddDeclareTransaction(BroadcastedDeclareTransactionInput),
    #[serde(rename = "starknet_addDeployAccountTransaction")]
    AddDeployAccountTransaction(BroadcastedDeployAccountTransactionInput),
    #[serde(rename = "starknet_addInvokeTransaction")]
    AddInvokeTransaction(BroadcastedInvokeTransactionInput),
    #[serde(rename = "starknet_estimateMessageFee")]
    EstimateMessageFee(EstimateMessageFeeRequestWrapper),
    #[serde(rename = "starknet_simulateTransactions")]
    SimulateTransactions(SimulateTransactionsInput),
    #[serde(rename = "starknet_traceTransaction")]
    TraceTransaction(TransactionHashInput),
    #[serde(rename = "starknet_traceBlockTransactions")]
    BlockTransactionTraces(BlockIdInput),
    #[serde(rename = "devnet_impersonateAccount")]
    ImpersonateAccount(AccountAddressInput),
    #[serde(rename = "devnet_stopImpersonateAccount")]
    StopImpersonateAccount(AccountAddressInput),
    #[serde(rename = "devnet_autoImpersonate", with = "empty_params")]
    AutoImpersonate,
    #[serde(rename = "devnet_stopAutoImpersonate", with = "empty_params")]
    StopAutoImpersonate,
    #[serde(rename = "devnet_dump", with = "optional_params")]
    Dump(Option<DumpPath>),
    #[serde(rename = "devnet_load")]
    Load(LoadPath),
    #[serde(rename = "devnet_postmanLoad")]
    PostmanLoadL1MessagingContract(PostmanLoadL1MessagingContract),
    #[serde(rename = "devnet_postmanFlush", with = "optional_params")]
    PostmanFlush(Option<FlushParameters>),
    #[serde(rename = "devnet_postmanSendMessageToL2")]
    PostmanSendMessageToL2(MessageToL2),
    #[serde(rename = "devnet_postmanConsumeMessageFromL2")]
    PostmanConsumeMessageFromL2(MessageToL1),
    #[serde(rename = "devnet_createBlock", with = "empty_params")]
    CreateBlock,
    #[serde(rename = "devnet_abortBlocks")]
    AbortBlocks(AbortingBlocks),
    #[serde(rename = "devnet_setGasPrice")]
    SetGasPrice(GasModificationRequest),
    #[serde(rename = "devnet_restart", with = "optional_params")]
    Restart(Option<RestartParameters>),
    #[serde(rename = "devnet_setTime")]
    SetTime(SetTime),
    #[serde(rename = "devnet_increaseTime")]
    IncreaseTime(IncreaseTime),
    #[serde(rename = "devnet_getPredeployedAccounts", with = "optional_params")]
    PredeployedAccounts(Option<PredeployedAccountsQuery>),
    #[serde(rename = "devnet_getAccountBalance")]
    AccountBalance(BalanceQuery),
    #[serde(rename = "devnet_mint")]
    Mint(MintTokensRequest),
    #[serde(rename = "devnet_getConfig", with = "empty_params")]
    DevnetConfig,
}

impl JsonRpcRequest {
    pub fn requires_notifying(&self) -> bool {
        #![warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_)
            | Self::PostmanFlush(_)
            | Self::PostmanSendMessageToL2(_)
            | Self::CreateBlock
            | Self::AbortBlocks(_)
            | Self::SetTime(_)
            | Self::IncreaseTime(_)
            | Self::Mint(_) => true,
            Self::SpecVersion
            | Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::ClassByHash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::ChainId
            | Self::Syncing
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::BlockTransactionTraces(_)
            | Self::ImpersonateAccount(_)
            | Self::StopImpersonateAccount(_)
            | Self::AutoImpersonate
            | Self::StopAutoImpersonate
            | Self::Dump(_)
            | Self::Load(_)
            | Self::PostmanLoadL1MessagingContract(_)
            | Self::PostmanConsumeMessageFromL2(_)
            | Self::SetGasPrice(_)
            | Self::Restart(_)
            | Self::PredeployedAccounts(_)
            | Self::AccountBalance(_)
            | Self::StorageProof(_)
            | Self::DevnetConfig => false,
        }
    }

    /// Should the request be retried by being forwarded to the forking origin?
    fn is_forwardable_to_origin(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::ClassByHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::StorageProof(_)
            | Self::BlockTransactionTraces(_) => true,
            Self::SpecVersion
            | Self::ChainId
            | Self::Syncing
            | Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_)
            | Self::ImpersonateAccount(_)
            | Self::StopImpersonateAccount(_)
            | Self::AutoImpersonate
            | Self::StopAutoImpersonate
            | Self::Dump(_)
            | Self::Load(_)
            | Self::PostmanLoadL1MessagingContract(_)
            | Self::PostmanFlush(_)
            | Self::PostmanSendMessageToL2(_)
            | Self::PostmanConsumeMessageFromL2(_)
            | Self::CreateBlock
            | Self::AbortBlocks(_)
            | Self::SetGasPrice(_)
            | Self::Restart(_)
            | Self::SetTime(_)
            | Self::IncreaseTime(_)
            | Self::PredeployedAccounts(_)
            | Self::AccountBalance(_)
            | Self::Mint(_)
            | Self::DevnetConfig => false,
        }
    }

    /// postmanFlush not dumped because it creates new RPC calls which get dumped
    fn is_dumpable(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::ImpersonateAccount(_)
            | Self::StopImpersonateAccount(_)
            | Self::AutoImpersonate
            | Self::StopAutoImpersonate
            | Self::PostmanLoadL1MessagingContract(_)
            | Self::PostmanSendMessageToL2(_)
            | Self::PostmanConsumeMessageFromL2(_)
            | Self::CreateBlock
            | Self::AbortBlocks(_)
            | Self::SetGasPrice(_)
            | Self::SetTime(_)
            | Self::IncreaseTime(_)
            | Self::Mint(_)
            | Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_) => true,
            Self::SpecVersion
            | Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::ClassByHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::ChainId
            | Self::Syncing
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::BlockTransactionTraces(_)
            | Self::Dump(_)
            | Self::Load(_)
            | Self::PostmanFlush(_)
            | Self::Restart(_)
            | Self::PredeployedAccounts(_)
            | Self::AccountBalance(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::StorageProof(_)
            | Self::DevnetConfig => false,
        }
    }
}

#[derive(Deserialize, AllVariantsSerdeRenames, VariantName)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "method", content = "params")]
pub enum JsonRpcSubscriptionRequest {
    #[serde(rename = "starknet_subscribeNewHeads", with = "optional_params")]
    NewHeads(Option<SubscriptionBlockIdInput>),
    #[serde(rename = "starknet_subscribeTransactionStatus")]
    TransactionStatus(TransactionHashInput),
    #[serde(rename = "starknet_subscribePendingTransactions", with = "optional_params")]
    PendingTransactions(Option<PendingTransactionsSubscriptionInput>),
    #[serde(rename = "starknet_subscribeEvents")]
    Events(Option<EventsSubscriptionInput>),
    #[serde(rename = "starknet_unsubscribe")]
    Unsubscribe(SubscriptionIdInput),
}

fn to_json_rpc_request<D>(call: &RpcMethodCall) -> Result<D, RpcError>
where
    D: DeserializeOwned,
{
    let params: serde_json::Value = call.params.clone().into();
    let deserializable_call = json!({
        "method": call.method,
        "params": params
    });

    serde_json::from_value::<D>(deserializable_call).map_err(|err| {
        let err = err.to_string();
        // since JSON-RPC specification requires returning a Method Not Found error,
        // we apply a hacky way to induce this - checking the stringified error message
        let distinctive_error = format!("unknown variant `{}`", call.method);
        if err.contains(&distinctive_error) {
            error!(target: "rpc", method = ?call.method, "failed to deserialize method due to unknown variant");
            RpcError::method_not_found()
        } else {
            error!(target: "rpc", method = ?call.method, ?err, "failed to deserialize method");
            RpcError::invalid_params(err)
        }
    })
}

impl std::fmt::Display for JsonRpcRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variant_name())
    }
}

#[derive(Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum JsonRpcResponse {
    Starknet(StarknetResponse),
    Devnet(DevnetResponse),
    Empty,
}

impl From<StarknetResponse> for JsonRpcResponse {
    fn from(resp: StarknetResponse) -> Self {
        JsonRpcResponse::Starknet(resp)
    }
}

impl From<DevnetResponse> for JsonRpcResponse {
    fn from(resp: DevnetResponse) -> Self {
        JsonRpcResponse::Devnet(resp)
    }
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum StarknetResponse {
    Block(Block),
    PendingBlock(PendingBlock),
    StateUpdate(StateUpdate),
    PendingStateUpdate(PendingStateUpdate),
    Felt(Felt),
    Transaction(TransactionWithHash),
    TransactionReceiptByTransactionHash(Box<TransactionReceipt>),
    TransactionStatusByHash(TransactionStatus),
    ContractClass(CodegenContractClass),
    CompiledCasm(CasmContractClass),
    BlockTransactionCount(u64),
    Call(Vec<Felt>),
    EstimateFee(Vec<FeeEstimateWrapper>),
    BlockNumber(BlockNumber),
    BlockHashAndNumber(BlockHashAndNumberOutput),
    String(String),
    Syncing(SyncingOutput),
    Events(EventsChunk),
    AddDeclareTransaction(DeclareTransactionOutput),
    AddDeployAccountTransaction(DeployAccountTransactionOutput),
    TransactionHash(TransactionHashOutput),
    EstimateMessageFee(FeeEstimateWrapper),
    SimulateTransactions(Vec<SimulatedTransaction>),
    TraceTransaction(TransactionTrace),
    BlockTransactionTraces(Vec<BlockTransactionTrace>),
    MessagesStatusByL1Hash(Vec<L1HandlerTransactionStatus>),
}

#[derive(Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum DevnetResponse {
    MessagingContractAddress(MessagingLoadAddress),
    FlushedMessages(FlushedMessages),
    MessageHash(MessageHash),
    CreatedBlock(CreatedBlock),
    AbortedBlocks(AbortedBlocks),
    GasModification(GasModification),
    SetTime(SetTimeResponse),
    IncreaseTime(IncreaseTimeResponse),
    TransactionHash(TransactionHashOutput),
    PredeployedAccounts(Vec<SerializableAccount>),
    AccountBalance(AccountBalanceResponse),
    MintTokens(MintTokensResponse),
    DevnetConfig(DevnetConfig),
    DevnetDump(DumpResponseBody),
}

#[cfg(test)]
mod requests_tests {

    use serde_json::json;
    use starknet_types::felt::felt_from_prefixed_hex;

    use super::JsonRpcRequest;
    use crate::rpc_core::request::RpcMethodCall;
    use crate::test_utils::assert_contains;

    #[test]
    fn deserialize_get_block_with_transaction_hashes_request() {
        let json_str =
            r#"{"method":"starknet_getBlockWithTxHashes","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pending"));

        assert_deserialization_fails(
            &json_str.replace("latest", "0x134134"),
            "Invalid block ID: unknown variant `0x134134`, expected `latest` or `pending`",
        );
    }

    #[test]
    fn deserialize_get_block_with_transactions_request() {
        let json_str = r#"{"method":"starknet_getBlockWithTxs","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pending"));

        assert_deserialization_fails(
            json_str.replace("latest", "0x134134").as_str(),
            "Invalid block ID: unknown variant `0x134134`, expected `latest` or `pending`",
        );
    }

    #[test]
    fn deserialize_get_state_update_request() {
        let json_str = r#"{"method":"starknet_getStateUpdate","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pending"));

        assert_deserialization_fails(
            &json_str.replace("latest", "0x134134"),
            "Invalid block ID: unknown variant `0x134134`, expected `latest` or `pending`",
        );
    }

    #[test]
    fn deserialize_get_storage_at_request() {
        let json_str = r#"{"method":"starknet_getStorageAt","params":{"contract_address":"0x134134","key":"0x134134","block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            &json_str.replace(r#""contract_address":"0x134134""#, r#""contract_address":"123""#),
            "Missing prefix 0x in 123",
        );

        assert_deserialization_fails(
            &json_str.replace(r#""contract_address":"0x134134""#, r#""contract_address": 123"#),
            "invalid type: integer `123`, expected a string",
        );
    }

    #[test]
    fn deserialize_get_transaction_by_hash_request() {
        let json_str = r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x134134"}}"#;

        let request = serde_json::from_str::<JsonRpcRequest>(json_str).unwrap();

        match request {
            JsonRpcRequest::TransactionByHash(input) => {
                assert!(input.transaction_hash == felt_from_prefixed_hex("0x134134").unwrap());
            }
            _ => panic!("Wrong request type"),
        }

        // Errored json, there is no object just string is passed
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":"0x134134"}"#,
            "invalid type: string \"0x134134\", expected struct",
        );
        // Errored json, hash is not prefixed with 0x
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"134134"}}"#,
            "expected hex string to be prefixed by '0x'",
        );
        // Errored json, hex longer than 64 chars; misleading error message coming from dependency
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x004134134134134134134134134134134134134134134134134134134134134134"}}"#,
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_transaction_by_block_and_index_request() {
        let json_str = r#"{"method":"starknet_getTransactionByBlockIdAndIndex","params":{"block_id":"latest","index":0}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace('0', "\"0x1\"").as_str(),
            "invalid type: string \"0x1\", expected u64",
        );
    }

    #[test]
    fn deserialize_get_transaction_receipt_request() {
        let json_str = r#"{"method":"starknet_getTransactionReceipt","params":{"transaction_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_class_request() {
        let json_str = r#"{"method":"starknet_getClass","params":{"block_id":"latest","class_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_class_hash_at_request() {
        let json_str = r#"{"method":"starknet_getClassHashAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "Error converting from hex string",
        );
    }

    #[test]
    fn deserialize_get_class_at_request() {
        let json_str = r#"{"method":"starknet_getClassAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0x", "").as_str(), "Missing prefix 0x");
    }

    #[test]
    fn deserialize_get_block_transaction_count_request() {
        let json_str =
            r#"{"method":"starknet_getBlockTransactionCount","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("latest", "0x134134").as_str(),
            "Invalid block ID: unknown variant `0x134134`, expected `latest` or `pending`",
        );
    }

    #[test]
    fn deserialize_call_request() {
        let json_str = r#"{
            "method":"starknet_call",
            "params":{
                "block_id":"latest",
                "request":{
                    "contract_address":"0xAAABB",
                    "entry_point_selector":"0x134134",
                    "calldata":["0x134134"]
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("starknet_call", "starknet_Call").as_str(),
            "unknown variant `starknet_Call`",
        );

        assert_deserialization_fails(
            json_str
                .replace(r#""contract_address":"0xAAABB""#, r#""contract_address":"123""#)
                .as_str(),
            "Error converting from hex string",
        );
        assert_deserialization_fails(
            json_str
                .replace(
                    r#""entry_point_selector":"0x134134""#,
                    r#""entry_point_selector":"134134""#,
                )
                .as_str(),
            "expected hex string to be prefixed by '0x'",
        );
        assert_deserialization_fails(
            json_str.replace(r#""calldata":["0x134134"]"#, r#""calldata":["123"]"#).as_str(),
            "expected hex string to be prefixed by '0x'",
        );
        assert_deserialization_fails(
            json_str.replace(r#""calldata":["0x134134"]"#, r#""calldata":[123]"#).as_str(),
            "invalid type: integer `123`",
        );
    }

    #[test]
    fn deserialize_deploy_account_fee_estimation_request() {
        let json_str = r#"{
            "method":"starknet_estimateFee",
            "params":{
                "block_id":"latest",
                "simulation_flags": [],
                "request":[
                    {
                        "type":"DEPLOY_ACCOUNT",
                        "max_fee": "0xA",
                        "version": "0x1",
                        "signature": ["0xFF", "0xAA"],
                        "nonce": "0x0",
                        "contract_address_salt": "0x01",
                        "constructor_calldata": ["0x01"],
                        "class_hash": "0x01"
                    }
                ]
            }
        }"#;

        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("estimateFee", "estimate_fee").as_str(),
            "unknown variant `starknet_estimate_fee`",
        );
    }

    fn sample_declare_v1_body() -> serde_json::Value {
        json!({
            "type": "DECLARE",
            "max_fee": "0xA",
            "version": "0x1",
            "signature": ["0xFF", "0xAA"],
            "nonce": "0x0",
            "sender_address": "0x0001",
            "contract_class": {
                "abi": [{
                    "inputs": [],
                    "name": "getPublicKey",
                    "outputs": [
                        {
                            "name": "publicKey",
                            "type": "felt"
                        }
                    ],
                    "stateMutability": "view",
                    "type": "function"
                },
                {
                    "inputs": [],
                    "name": "setPublicKey",
                    "outputs": [
                        {
                            "name": "publicKey",
                            "type": "felt"
                        }
                    ],
                    "type": "function"
                }],
                "program": "",
                "entry_points_by_type": {
                    "CONSTRUCTOR": [],
                    "EXTERNAL": [],
                    "L1_HANDLER": []
                }
            }
        })
    }

    fn sample_declare_v2_body() -> serde_json::Value {
        json!({
            "type":"DECLARE",
            "max_fee": "0xde0b6b3a7640000",
            "version": "0x2",
            "signature": [
                "0x2216f8f4d9abc06e130d2a05b13db61850f0a1d21891c7297b98fd6cc51920d",
                "0x6aadfb198bbffa8425801a2342f5c6d804745912114d5976f53031cd789bb6d"
            ],
            "nonce": "0x0",
            "compiled_class_hash":"0x63b33a5f2f46b1445d04c06d7832c48c48ad087ce0803b71f2b8d96353716ca",
            "sender_address":"0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "contract_class": {
                "sierra_program": ["0xAA", "0xBB"],
                "entry_points_by_type": {
                    "EXTERNAL": [{"function_idx":0,"selector":"0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320"},{"function_idx":1,"selector":"0x39e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695"}],
                    "L1_HANDLER": [],
                    "CONSTRUCTOR": [{"function_idx":2,"selector":"0x28ffe4ff0f226a9107253e17a904099aa4f63a02a5621de0576e5aa71bc5194"}]
                },
                "abi": "[{\"type\": \"function\", \"name\": \"constructor\", \"inputs\": [{\"name\": \"initial_balance\", \"type\": \"core::felt252\"}], \"outputs\": [], \"state_mutability\": \"external\"}, {\"type\": \"function\", \"name\": \"increase_balance\", \"inputs\": [{\"name\": \"amount1\", \"type\": \"core::felt252\"}, {\"name\": \"amount2\", \"type\": \"core::felt252\"}], \"outputs\": [], \"state_mutability\": \"external\"}, {\"type\": \"function\", \"name\": \"get_balance\", \"inputs\": [], \"outputs\": [{\"type\": \"core::felt252\"}], \"state_mutability\": \"view\"}]",
                "contract_class_version": "0.1.0"
            }
        })
    }

    fn create_declare_request(tx: serde_json::Value) -> serde_json::Value {
        json!({
            "method":"starknet_addDeclareTransaction",
            "params":{
                "declare_transaction": tx
            }
        })
    }

    fn create_estimate_request(requests: &[serde_json::Value]) -> serde_json::Value {
        json!({
            "method": "starknet_estimateFee",
            "params": {
                "block_id": "latest",
                "simulation_flags": [],
                "request": requests
            }
        })
    }

    #[test]
    fn deserialize_declare_v1_fee_estimation_request() {
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v1_body()]).to_string(),
        );
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v1_body()]).to_string().replace(
                r#""version": "0x1""#,
                r#""version": "0x100000000000000000000000000000001""#,
            ),
        );
        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace(r#""version":"0x1""#, r#""version":"0x123""#),
            "Invalid version of declare transaction: \"0x123\"",
        );
        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace(r#""version":"0x1""#, r#""version":"0x2""#),
            "Invalid declare transaction v2",
        );
    }

    #[test]
    fn deserialize_declare_v2_fee_estimation_request() {
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v2_body()]).to_string(),
        );
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v2_body()]).to_string().replace(
                r#""version":"0x2""#,
                r#""version":"0x100000000000000000000000000000002""#,
            ),
        );
        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v2_body()])
                .to_string()
                .replace(r#""version":"0x2""#, r#""version":"0x123""#),
            "Invalid version of declare transaction: \"0x123\"",
        );
        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v2_body()])
                .to_string()
                .replace(r#""version":"0x2""#, r#""version":"0x1""#),
            "Invalid declare transaction v1",
        );
    }

    #[test]
    fn deserialize_get_events_request() {
        let json_str = r#"{
            "method":"starknet_getEvents",
            "params":{
                "filter":{
                    "chunk_size": 1,
                    "address":"0xAAABB",
                    "keys":[["0xFF"], ["0xAA"]],
                    "from_block": "latest",
                    "to_block": "pending",
                    "continuation_token": "0x11"
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(json_str.replace(r#""to_block": "pending","#, "").as_str());

        assert_deserialization_fails(
            json_str.replace(r#""chunk_size": 1,"#, "").as_str(),
            "missing field `chunk_size`",
        );
    }

    #[test]
    fn deserialize_get_nonce_request() {
        let json_str = r#"{
            "method":"starknet_getNonce",
            "params":{
                "block_id":"latest",
                "contract_address":"0xAAABB"
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_fails(
            json_str.replace(r#""block_id":"latest","#, "").as_str(),
            "missing field `block_id`",
        );
    }

    #[test]
    fn deserialize_add_deploy_account_transaction_request() {
        let json_str = r#"{
            "method":"starknet_addDeployAccountTransaction",
            "params":{
                "deploy_account_transaction":{
                    "type":"DEPLOY_ACCOUNT",
                    "max_fee": "0xA",
                    "version": "0x1",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "contract_address_salt": "0x01",
                    "class_hash": "0x01",
                    "constructor_calldata": ["0x01"]
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_fails(
            json_str.replace(r#""class_hash": "0x01","#, "").as_str(),
            "missing field `class_hash`",
        );
    }

    #[test]
    fn deserialize_add_declare_transaction_v1_request() {
        assert_deserialization_succeeds(
            &create_declare_request(sample_declare_v1_body()).to_string(),
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace(r#""version":"0x1""#, r#""version":"0x2""#),
            "Invalid declare transaction v2",
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace(r#""version":"0x1""#, r#""version":123"#),
            "Invalid version of declare transaction: 123",
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace(r#""name":"publicKey""#, r#""name":123"#),
            "Invalid declare transaction v1: Invalid function ABI entry: invalid type: number, \
             expected a string",
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v1_body()])
                .to_string()
                .replace("max_fee", "maxFee"),
            "Invalid declare transaction v1: missing field `max_fee`",
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v1_body())
                .to_string()
                .replace(r#""nonce":"0x0""#, r#""nonce":123"#),
            "Invalid declare transaction v1: invalid type: integer `123`",
        );
    }

    #[test]
    fn deserialize_add_declare_transaction_v2_request() {
        assert_deserialization_succeeds(
            &create_declare_request(sample_declare_v2_body()).to_string(),
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v2_body())
                .to_string()
                .replace(r#""version":"0x2""#, r#""version":"0x123""#),
            "Invalid version of declare transaction: \"0x123\"",
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v2_body())
                .to_string()
                .replace(r#""version":"0x2""#, r#""version":"0x1""#),
            "Invalid declare transaction v1",
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v2_body()])
                .to_string()
                .replace("max_fee", "maxFee"),
            "Invalid declare transaction v2: missing field `max_fee`",
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v2_body())
                .to_string()
                .replace(r#""nonce":"0x0""#, r#""nonce":123"#),
            "Invalid declare transaction v2: invalid type: integer `123`",
        );
    }

    #[test]
    fn deseralize_chain_id_request() {
        for body in [
            json!({
                "method": "starknet_chainId",
                "params": {}
            }),
            json!({
                "method": "starknet_chainId",
                "params": []
            }),
            json!({
                "method": "starknet_chainId",
            }),
        ] {
            assert_deserialization_succeeds(body.to_string().as_str())
        }
    }

    #[test]
    fn deserialize_spec_version_request() {
        for body in [
            json!({
                "method": "starknet_specVersion",
                "params": {}
            }),
            json!({
                "method": "starknet_specVersion",
                "params": []
            }),
            json!({
                "method": "starknet_specVersion",
            }),
        ] {
            assert_deserialization_succeeds(body.to_string().as_str())
        }
    }

    #[test]
    fn deserialize_devnet_methods_with_optional_body() {
        for mut body in [
            json!({
                "method": "devnet_dump",
                "params": {}
            }),
            json!({
                "method":"devnet_dump",
            }),
            json!({
                "method":"devnet_dump",
                "params": {"path": "path"}
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
                "params": {"with_balance": true}
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
                "params": {}
            }),
            json!({
                "method":"devnet_postmanFlush",
                "params": {"dry_run": true}
            }),
            json!({
                "method":"devnet_postmanFlush",
            }),
            json!({
                "method":"devnet_postmanFlush",
                "params": {}
            }),
        ] {
            let mut json_rpc_object = json!({
                "jsonrpc": "2.0",
                "id": 1,
            });

            json_rpc_object.as_object_mut().unwrap().append(body.as_object_mut().unwrap());

            let RpcMethodCall { method, params, .. } =
                serde_json::from_value(json_rpc_object).unwrap();
            let params: serde_json::Value = params.into();
            let deserializable_call = json!({
                "method": &method,
                "params": params
            });

            assert_deserialization_succeeds(deserializable_call.to_string().as_str())
        }
    }

    fn assert_deserialization_succeeds(json_str: &str) {
        serde_json::from_str::<JsonRpcRequest>(json_str).unwrap();
    }

    fn assert_deserialization_fails(json_str: &str, expected_msg: &str) {
        match serde_json::from_str::<JsonRpcRequest>(json_str) {
            Err(err) => assert_contains(&err.to_string(), expected_msg),
            other => panic!("Invalid result: {other:?}"),
        }
    }
}

#[cfg(test)]
mod response_tests {
    use crate::api::json_rpc::error::StrictRpcResult;
    use crate::api::json_rpc::ToRpcResponseResult;

    #[test]
    fn serializing_starknet_response_empty_variant_yields_empty_json_on_conversion_to_rpc_result() {
        assert_eq!(
            r#"{"result":{}}"#,
            serde_json::to_string(
                &StrictRpcResult::Ok(crate::api::json_rpc::JsonRpcResponse::Empty).to_rpc_result()
            )
            .unwrap()
        );
    }
}
