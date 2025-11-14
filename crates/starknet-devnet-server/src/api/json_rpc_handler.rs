use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use starknet_core::StarknetBlock;
use starknet_core::starknet::starknet_config::DumpOn;
use starknet_types::emitted_event::SubscriptionEmittedEvent;
use starknet_types::rpc::block::{BlockId, BlockTag, ReorgData};
use starknet_types::rpc::transactions::TransactionFinalityStatus;
use tokio::sync::Mutex;
use tracing::{info, trace};

use crate::api::models::{
    AccountAddressInput, BlockAndClassHashInput, BlockAndContractAddressInput, BlockAndIndexInput,
    BlockIdInput, BroadcastedDeclareTransactionEnumWrapper, BroadcastedDeclareTransactionInput,
    BroadcastedDeployAccountTransactionEnumWrapper, BroadcastedDeployAccountTransactionInput,
    BroadcastedInvokeTransactionEnumWrapper, BroadcastedInvokeTransactionInput, CallInput,
    ClassHashInput, DevnetSpecRequest, EstimateFeeInput, EventsInput, GetStorageInput,
    JsonRpcRequest, JsonRpcResponse, JsonRpcWsRequest, LoadPath, SimulateTransactionsInput,
    StarknetSpecRequest, ToRpcResponseResult, TransactionHashInput, to_json_rpc_request,
};
use crate::api::origin_forwarder::OriginForwarder;
use crate::api::{Api, ApiError, error};
use crate::dump_util::dump_event;
use crate::restrictive_mode::is_json_rpc_method_restricted;
use crate::rpc_core;
use crate::rpc_core::error::{ErrorCode, RpcError};
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::{ResponseResult, RpcResponse};
use crate::rpc_handler::RpcHandler;
use crate::subscribe::{
    NewTransactionNotification, NewTransactionReceiptNotification, NewTransactionStatus,
    NotificationData, SocketId,
};

/// This object will be used as a shared state between HTTP calls.
/// Is similar to the HttpApiHandler but is with extended functionality and is used for JSON-RPC
/// methods
#[derive(Clone)]
pub struct JsonRpcHandler {
    pub api: Api,
    pub origin_caller: Option<OriginForwarder>,
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

        if !self.allows_method(&original_call.method) {
            return ResponseResult::Error(RpcError::new(ErrorCode::MethodForbidden));
        }

        let is_request_forwardable = request.is_forwardable_to_origin(); // applicable if forking
        let is_request_dumpable = request.is_dumpable();

        // for later comparison and subscription notifications
        let old_latest_block = if request.requires_notifying() {
            Some(self.get_block_by_tag(BlockTag::Latest).await)
        } else {
            None
        };

        let old_pre_confirmed_block =
            if request.requires_notifying() && self.api.config.uses_pre_confirmed_block() {
                Some(self.get_block_by_tag(BlockTag::PreConfirmed).await)
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

        if let Err(e) = self.broadcast_changes(old_latest_block, old_pre_confirmed_block).await {
            return ResponseResult::Error(e.api_error_to_rpc_error());
        }

        starknet_resp.to_rpc_result()
    }

    async fn on_call(&self, call: RpcMethodCall) -> RpcResponse {
        let id = call.id.clone();
        trace!(target: "rpc",  id = ?id, method = ?call.method, "received method call");

        match to_json_rpc_request(&call) {
            Ok(req) => {
                let result = self.on_request(req, call).await;
                RpcResponse::new(id, result)
            }
            Err(e) => RpcResponse::from_rpc_error(e, id),
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

        self.api.sockets.lock().await.remove(&socket_id);
        if socket_safely_closed {
            tracing::info!("Websocket disconnected");
        } else {
            tracing::error!("Failed socket read");
        }
    }
}

impl JsonRpcHandler {
    pub fn new(api: Api) -> JsonRpcHandler {
        let origin_caller = if let (Some(url), Some(block_number)) =
            (&api.config.fork_config.url, api.config.fork_config.block_number)
        {
            Some(OriginForwarder::new(url.clone(), block_number))
        } else {
            None
        };

        JsonRpcHandler { api, origin_caller }
    }

    /// The latest and pre_confirmed block are always defined, so to avoid having to deal with
    /// Err/None in places where this method is called, it is defined to return an empty
    /// accepted block, even though that case should never happen.
    async fn get_block_by_tag(&self, tag: BlockTag) -> StarknetBlock {
        let starknet = self.api.starknet.lock().await;
        match starknet.get_block(&BlockId::Tag(tag)) {
            Ok(block) => block.clone(),
            _ => StarknetBlock::create_empty_accepted(),
        }
    }

    async fn broadcast_pre_confirmed_tx_changes(
        &self,
        old_pre_confirmed_block: StarknetBlock,
    ) -> Result<(), error::ApiError> {
        let new_pre_confirmed_block = self.get_block_by_tag(BlockTag::PreConfirmed).await;
        let old_pre_confirmed_txs = old_pre_confirmed_block.get_transactions();
        let new_pre_confirmed_txs = new_pre_confirmed_block.get_transactions();

        if new_pre_confirmed_txs.len() > old_pre_confirmed_txs.len() {
            #[allow(clippy::expect_used)]
            let new_tx_hash = new_pre_confirmed_txs.last().expect("has at least one element");

            let mut notifications = vec![];
            let starknet = self.api.starknet.lock().await;

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
            notifications.push(NotificationData::NewTransaction(NewTransactionNotification {
                tx: tx.clone(),
                finality_status: TransactionFinalityStatus::PreConfirmed,
            }));

            let receipt = starknet
                .get_transaction_receipt_by_hash(new_tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;

            notifications.push(NotificationData::NewTransactionReceipt(
                NewTransactionReceiptNotification {
                    tx_receipt: receipt,
                    sender_address: tx.get_sender_address(),
                },
            ));

            let events = starknet.get_unlimited_events(
                Some(BlockId::Tag(BlockTag::PreConfirmed)),
                Some(BlockId::Tag(BlockTag::PreConfirmed)),
                None,
                None,
                None, // pre-confirmed block only has pre-confirmed txs
            )?;

            drop(starknet); // Drop immediately after last use

            for emitted_event in events.into_iter().filter(|e| &e.transaction_hash == new_tx_hash) {
                notifications.push(NotificationData::Event(SubscriptionEmittedEvent {
                    emitted_event,
                    finality_status: TransactionFinalityStatus::PreConfirmed,
                }));
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

        let finality_status = TransactionFinalityStatus::AcceptedOnL2;
        let latest_txs = new_latest_block.get_transactions();
        for tx_hash in latest_txs {
            let tx = starknet
                .get_transaction_by_hash(*tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;
            notifications.push(NotificationData::NewTransaction(NewTransactionNotification {
                tx: tx.clone(),
                finality_status,
            }));

            let status = starknet
                .get_transaction_execution_and_finality_status(*tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;
            notifications.push(NotificationData::TransactionStatus(NewTransactionStatus {
                transaction_hash: *tx_hash,
                status,
            }));

            let tx_receipt = starknet
                .get_transaction_receipt_by_hash(tx_hash)
                .map_err(error::ApiError::StarknetDevnetError)?;
            notifications.push(NotificationData::NewTransactionReceipt(
                NewTransactionReceiptNotification {
                    tx_receipt,
                    sender_address: tx.get_sender_address(),
                },
            ));
        }

        let events = starknet.get_unlimited_events(
            Some(BlockId::Tag(BlockTag::Latest)),
            Some(BlockId::Tag(BlockTag::Latest)),
            None,
            None,
            None, // latest block only has txs accepted on L2
        )?;

        drop(starknet); // Drop immediately after last use

        for emitted_event in events {
            notifications.push(NotificationData::Event(SubscriptionEmittedEvent {
                emitted_event,
                finality_status,
            }));
        }

        self.api.sockets.lock().await.notify_subscribers(&notifications).await;
        Ok(())
    }

    /// Notify subscribers of what they are subscribed to.
    async fn broadcast_changes(
        &self,
        old_latest_block: Option<StarknetBlock>,
        old_pre_confirmed_block: Option<StarknetBlock>,
    ) -> Result<(), error::ApiError> {
        let Some(old_latest_block) = old_latest_block else {
            return Ok(());
        };

        if let Some(old_pre_confirmed_block) = old_pre_confirmed_block {
            self.broadcast_pre_confirmed_tx_changes(old_pre_confirmed_block).await?;
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
    ) -> Result<(), ApiError> {
        let last_aborted_block_hash =
            *self.api.starknet.lock().await.last_aborted_block_hash().ok_or(
                ApiError::StarknetDevnetError(
                    starknet_core::error::Error::UnexpectedInternalError {
                        msg: "Aborted block hash should be defined.".into(),
                    },
                ),
            )?;

        let notification = NotificationData::Reorg(ReorgData {
            starting_block_hash: last_aborted_block_hash,
            starting_block_number: new_latest_block.block_number().unchecked_next(),
            ending_block_hash: old_latest_block.block_hash(),
            ending_block_number: old_latest_block.block_number(),
        });

        self.api.sockets.lock().await.notify_subscribers(&[notification]).await;
        Ok(())
    }

    /// Matches the request to the corresponding enum variant and executes the request.
    async fn execute(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, error::ApiError> {
        trace!(target: "JsonRpcHandler::execute", "executing request");
        match req {
            JsonRpcRequest::StarknetSpecRequest(req) => self.execute_starknet_spec(req).await,
            JsonRpcRequest::DevnetSpecRequest(req) => self.execute_devnet_spec(req).await,
        }
    }

    async fn execute_starknet_spec(
        &self,
        req: StarknetSpecRequest,
    ) -> Result<JsonRpcResponse, error::ApiError> {
        match req {
            StarknetSpecRequest::SpecVersion => self.spec_version(),
            StarknetSpecRequest::BlockWithTransactionHashes(block) => {
                self.get_block_with_tx_hashes(block.block_id).await
            }
            StarknetSpecRequest::BlockWithFullTransactions(block) => {
                self.get_block_with_txs(block.block_id).await
            }
            StarknetSpecRequest::BlockWithReceipts(block) => {
                self.get_block_with_receipts(block.block_id).await
            }
            StarknetSpecRequest::StateUpdate(block) => self.get_state_update(block.block_id).await,
            StarknetSpecRequest::StorageAt(GetStorageInput { contract_address, key, block_id }) => {
                self.get_storage_at(contract_address, key, block_id).await
            }
            StarknetSpecRequest::TransactionStatusByHash(TransactionHashInput {
                transaction_hash,
            }) => self.get_transaction_status_by_hash(transaction_hash).await,
            StarknetSpecRequest::TransactionByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_by_hash(transaction_hash).await
            }
            StarknetSpecRequest::TransactionByBlockAndIndex(BlockAndIndexInput {
                block_id,
                index,
            }) => self.get_transaction_by_block_id_and_index(block_id, index).await,
            StarknetSpecRequest::TransactionReceiptByTransactionHash(TransactionHashInput {
                transaction_hash,
            }) => self.get_transaction_receipt_by_hash(transaction_hash).await,
            StarknetSpecRequest::ClassByHash(BlockAndClassHashInput { block_id, class_hash }) => {
                self.get_class(block_id, class_hash).await
            }
            StarknetSpecRequest::CompiledCasmByClassHash(ClassHashInput { class_hash }) => {
                self.get_compiled_casm(class_hash).await
            }
            StarknetSpecRequest::ClassHashAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_hash_at(block_id, contract_address).await,
            StarknetSpecRequest::ClassAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_at(block_id, contract_address).await,
            StarknetSpecRequest::BlockTransactionCount(block) => {
                self.get_block_txs_count(block.block_id).await
            }
            StarknetSpecRequest::Call(CallInput { request, block_id }) => {
                self.call(block_id, request).await
            }
            StarknetSpecRequest::EstimateFee(EstimateFeeInput {
                request,
                block_id,
                simulation_flags,
            }) => self.estimate_fee(block_id, request, simulation_flags).await,
            StarknetSpecRequest::BlockNumber => self.block_number().await,
            StarknetSpecRequest::BlockHashAndNumber => self.block_hash_and_number().await,
            StarknetSpecRequest::ChainId => self.chain_id().await,
            StarknetSpecRequest::Syncing => self.syncing().await,
            StarknetSpecRequest::Events(EventsInput { filter }) => self.get_events(filter).await,
            StarknetSpecRequest::ContractNonce(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_nonce(block_id, contract_address).await,
            StarknetSpecRequest::AddDeclareTransaction(BroadcastedDeclareTransactionInput {
                declare_transaction,
            }) => {
                let BroadcastedDeclareTransactionEnumWrapper::Declare(broadcasted_transaction) =
                    declare_transaction;
                self.add_declare_transaction(broadcasted_transaction).await
            }
            StarknetSpecRequest::AddDeployAccountTransaction(
                BroadcastedDeployAccountTransactionInput { deploy_account_transaction },
            ) => {
                let BroadcastedDeployAccountTransactionEnumWrapper::DeployAccount(
                    broadcasted_transaction,
                ) = deploy_account_transaction;
                self.add_deploy_account_transaction(broadcasted_transaction).await
            }
            StarknetSpecRequest::AddInvokeTransaction(BroadcastedInvokeTransactionInput {
                invoke_transaction,
            }) => {
                let BroadcastedInvokeTransactionEnumWrapper::Invoke(broadcasted_transaction) =
                    invoke_transaction;
                self.add_invoke_transaction(broadcasted_transaction).await
            }
            StarknetSpecRequest::EstimateMessageFee(request) => {
                self.estimate_message_fee(request.get_block_id(), request.get_raw_message().clone())
                    .await
            }
            StarknetSpecRequest::SimulateTransactions(SimulateTransactionsInput {
                block_id,
                transactions,
                simulation_flags,
            }) => self.simulate_transactions(block_id, transactions, simulation_flags).await,
            StarknetSpecRequest::TraceTransaction(TransactionHashInput { transaction_hash }) => {
                self.get_trace_transaction(transaction_hash).await
            }
            StarknetSpecRequest::BlockTransactionTraces(BlockIdInput { block_id }) => {
                self.get_trace_block_transactions(block_id).await
            }
            StarknetSpecRequest::MessagesStatusByL1Hash(data) => {
                self.get_messages_status(data).await
            }
            StarknetSpecRequest::StorageProof(data) => self.get_storage_proof(data).await,
        }
    }

    async fn execute_devnet_spec(
        &self,
        req: DevnetSpecRequest,
    ) -> Result<JsonRpcResponse, error::ApiError> {
        match req {
            DevnetSpecRequest::ImpersonateAccount(AccountAddressInput { account_address }) => {
                self.impersonate_account(account_address).await
            }
            DevnetSpecRequest::StopImpersonateAccount(AccountAddressInput { account_address }) => {
                self.stop_impersonating_account(account_address).await
            }
            DevnetSpecRequest::AutoImpersonate => self.set_auto_impersonate(true).await,
            DevnetSpecRequest::StopAutoImpersonate => self.set_auto_impersonate(false).await,
            DevnetSpecRequest::Dump(path) => self.dump(path).await,
            DevnetSpecRequest::Load(LoadPath { path }) => self.load(path).await,
            DevnetSpecRequest::PostmanLoadL1MessagingContract(data) => {
                self.postman_load(data).await
            }
            DevnetSpecRequest::PostmanFlush(data) => self.postman_flush(data).await,
            DevnetSpecRequest::PostmanSendMessageToL2(message) => {
                self.postman_send_message_to_l2(message).await
            }
            DevnetSpecRequest::PostmanConsumeMessageFromL2(message) => {
                self.postman_consume_message_from_l2(message).await
            }
            DevnetSpecRequest::CreateBlock => self.create_block().await,
            DevnetSpecRequest::AbortBlocks(data) => self.abort_blocks(data).await,
            DevnetSpecRequest::AcceptOnL1(data) => self.accept_on_l1(data).await,
            DevnetSpecRequest::SetGasPrice(data) => self.set_gas_price(data).await,
            DevnetSpecRequest::Restart(data) => self.restart(data).await,
            DevnetSpecRequest::SetTime(data) => self.set_time(data).await,
            DevnetSpecRequest::IncreaseTime(data) => self.increase_time(data).await,
            DevnetSpecRequest::PredeployedAccounts(data) => {
                self.get_predeployed_accounts(data).await
            }
            DevnetSpecRequest::AccountBalance(data) => self.get_account_balance(data).await,
            DevnetSpecRequest::Mint(data) => self.mint(data).await,
            DevnetSpecRequest::DevnetConfig => self.get_devnet_config().await,
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
                Err(e) => serde_json::to_string(&RpcResponse::from_rpc_error(e, rpc_call.id))
                    .unwrap_or_default(),
            },
            Err(e) => serde_json::to_string(&RpcResponse::from_rpc_error(
                RpcError::parse_error(e.to_string()),
                rpc_core::request::Id::Null,
            ))
            .unwrap_or_default(),
        };

        if let Err(e) = ws.lock().await.send(Message::Text(error_serialized.into())).await {
            tracing::error!("Error sending websocket message: {e}");
        }
    }

    fn allows_method(&self, method: &str) -> bool {
        if let Some(restricted_methods) = &self.api.server_config.restricted_methods {
            if is_json_rpc_method_restricted(method, restricted_methods) {
                return false;
            }
        }

        true
    }

    /// Since some subscriptions might need to send multiple messages, sending messages other than
    /// errors is left to individual RPC method handlers and this method returns an empty successful
    /// Result. A one-time request also returns an empty successful result, but actually sends the
    /// message.
    async fn on_websocket_rpc_call(
        &self,
        call: &RpcMethodCall,
        socket_id: SocketId,
    ) -> Result<(), RpcError> {
        trace!(target: "rpc",  id = ?call.id, method = ?call.method, "received websocket call");

        let req: JsonRpcWsRequest = to_json_rpc_request(call)?;
        match req {
            JsonRpcWsRequest::OneTimeRequest(req) => {
                let resp_result = self.on_request(*req, call.clone()).await;
                let mut sockets = self.api.sockets.lock().await;

                let socket_context =
                    sockets.get_mut(&socket_id).map_err(|e| e.api_error_to_rpc_error())?;

                match resp_result {
                    ResponseResult::Success(result_value) => {
                        socket_context.send_rpc_response(result_value, call.id.clone()).await;
                        Ok(())
                    }
                    ResponseResult::Error(rpc_error) => Err(rpc_error),
                }
            }
            JsonRpcWsRequest::SubscriptionRequest(req) => self
                .execute_ws_subscription(req, call.id.clone(), socket_id)
                .await
                .map_err(|e| e.api_error_to_rpc_error()),
        }
    }

    async fn update_dump(&self, event: &RpcMethodCall) -> Result<(), RpcError> {
        match self.api.config.dump_on {
            Some(DumpOn::Block) => {
                let dump_path = self
                    .api
                    .config
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
