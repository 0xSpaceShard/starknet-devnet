mod endpoints;
pub mod error;
pub mod models;
pub(crate) mod origin_forwarder;
#[cfg(test)]
mod spec_reader;
mod write_endpoints;

pub const RPC_SPEC_VERSION: &str = "0.7.1";

use models::{
    BlockAndClassHashInput, BlockAndContractAddressInput, BlockAndIndexInput, CallInput,
    EstimateFeeInput, EventsInput, GetStorageInput, TransactionHashInput,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet_rs_core::types::ContractClass as CodegenContractClass;
use starknet_types::felt::Felt;
use starknet_types::rpc::block::Block;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::state::StateUpdate;
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::{
    BlockTransactionTrace, EventsChunk, SimulatedTransaction, TransactionTrace, TransactionWithHash,
};
use starknet_types::starknet_api::block::BlockNumber;
use tracing::{error, info, trace};

use self::error::StrictRpcResult;
use self::models::{
    BlockHashAndNumberOutput, BlockIdInput, BroadcastedDeclareTransactionInput,
    BroadcastedDeployAccountTransactionInput, BroadcastedInvokeTransactionInput,
    ContractAddressInput, DeclareTransactionOutput, DeployAccountTransactionOutput,
    InvokeTransactionOutput, SyncingOutput, TransactionStatusOutput,
};
use self::origin_forwarder::OriginForwarder;
use super::Api;
use crate::api::json_rpc::models::{
    BroadcastedDeclareTransactionEnumWrapper, BroadcastedDeployAccountTransactionEnumWrapper,
    BroadcastedInvokeTransactionEnumWrapper, SimulateTransactionsInput,
};
use crate::api::serde_helpers::empty_params;
use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::ResponseResult;
use crate::rpc_handler::RpcHandler;

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
            Ok(StarknetResponse::Empty) => to_rpc_result(json!({})),
            Ok(data) => to_rpc_result(data),
            Err(err) => err.api_error_to_rpc_error().into(),
        }
    }
}

/// This object will be used as a shared state between HTTP calls.
/// Is simillar to the HttpApiHandler but is with extended functionality and is used for JSON-RPC
/// methods
#[derive(Clone)]
pub struct JsonRpcHandler {
    pub api: Api,
    pub origin_caller: Option<OriginForwarder>,
}

#[async_trait::async_trait]
impl RpcHandler for JsonRpcHandler {
    type Request = StarknetRequest;

    async fn on_request(
        &self,
        request: Self::Request,
        original_call: RpcMethodCall,
    ) -> ResponseResult {
        info!(target: "rpc", "received method in on_request {}", request);
        self.execute(request, original_call).await
    }
}

impl JsonRpcHandler {
    /// The method matches the request to the corresponding enum variant and executes the request
    async fn execute(
        &self,
        request: StarknetRequest,
        original_call: RpcMethodCall,
    ) -> ResponseResult {
        trace!(target: "JsonRpcHandler::execute", "executing starknet request");

        // true if origin should be tried after request fails; relevant in forking mode
        let mut forwardable = true;

        let starknet_resp = match request {
            StarknetRequest::SpecVersion => self.spec_version(),
            StarknetRequest::BlockWithTransactionHashes(block) => {
                self.get_block_with_tx_hashes(block.block_id).await
            }
            StarknetRequest::BlockWithFullTransactions(block) => {
                self.get_block_with_txs(block.block_id).await
            }
            StarknetRequest::BlockWithReceipts(block) => {
                self.get_block_with_receipts(block.block_id).await
            }
            StarknetRequest::StateUpdate(block) => self.get_state_update(block.block_id).await,
            StarknetRequest::StorageAt(GetStorageInput { contract_address, key, block_id }) => {
                self.get_storage_at(contract_address, key, block_id).await
            }
            StarknetRequest::TransactionStatusByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_status_by_hash(transaction_hash).await
            }
            StarknetRequest::TransactionByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_by_hash(transaction_hash).await
            }
            StarknetRequest::TransactionByBlockAndIndex(BlockAndIndexInput { block_id, index }) => {
                self.get_transaction_by_block_id_and_index(block_id, index).await
            }
            StarknetRequest::TransactionReceiptByTransactionHash(TransactionHashInput {
                transaction_hash,
            }) => self.get_transaction_receipt_by_hash(transaction_hash).await,
            StarknetRequest::ClassByHash(BlockAndClassHashInput { block_id, class_hash }) => {
                self.get_class(block_id, class_hash).await
            }
            StarknetRequest::ClassHashAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_hash_at(block_id, contract_address).await,
            StarknetRequest::ClassAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_at(block_id, contract_address).await,
            StarknetRequest::BlockTransactionCount(block) => {
                self.get_block_txs_count(block.block_id).await
            }
            StarknetRequest::Call(CallInput { request, block_id }) => {
                self.call(block_id, request).await
            }
            StarknetRequest::EsimateFee(EstimateFeeInput {
                request,
                block_id,
                simulation_flags,
            }) => self.estimate_fee(block_id, request, simulation_flags).await,
            StarknetRequest::BlockNumber => self.block_number().await,
            StarknetRequest::BlockHashAndNumber => self.block_hash_and_number().await,
            StarknetRequest::ChainId => self.chain_id().await,
            StarknetRequest::Syncing => self.syncing().await,
            StarknetRequest::Events(EventsInput { filter }) => self.get_events(filter).await,
            StarknetRequest::ContractNonce(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_nonce(block_id, contract_address).await,
            StarknetRequest::AddDeclareTransaction(BroadcastedDeclareTransactionInput {
                declare_transaction,
            }) => {
                let BroadcastedDeclareTransactionEnumWrapper::Declare(broadcasted_transaction) =
                    declare_transaction;
                self.add_declare_transaction(broadcasted_transaction).await
            }
            StarknetRequest::AddDeployAccountTransaction(
                BroadcastedDeployAccountTransactionInput { deploy_account_transaction },
            ) => {
                forwardable = false;
                let BroadcastedDeployAccountTransactionEnumWrapper::DeployAccount(
                    broadcasted_transaction,
                ) = deploy_account_transaction;
                self.add_deploy_account_transaction(broadcasted_transaction).await
            }
            StarknetRequest::AddInvokeTransaction(BroadcastedInvokeTransactionInput {
                invoke_transaction,
            }) => {
                let BroadcastedInvokeTransactionEnumWrapper::Invoke(broadcasted_transaction) =
                    invoke_transaction;
                self.add_invoke_transaction(broadcasted_transaction).await
            }
            StarknetRequest::EstimateMessageFee(request) => {
                self.estimate_message_fee(request.get_block_id(), request.get_raw_message().clone())
                    .await
            }
            StarknetRequest::SimulateTransactions(SimulateTransactionsInput {
                block_id,
                transactions,
                simulation_flags,
            }) => self.simulate_transactions(block_id, transactions, simulation_flags).await,
            StarknetRequest::TraceTransaction(TransactionHashInput { transaction_hash }) => {
                self.get_trace_transaction(transaction_hash).await
            }
            StarknetRequest::BlockTransactionTraces(BlockIdInput { block_id }) => {
                self.get_trace_block_transactions(block_id).await
            }
            StarknetRequest::ImpersonateAccount(ContractAddressInput { contract_address }) => {
                self.impersonate_account(contract_address).await
            }
            StarknetRequest::StopImpersonateAccount(ContractAddressInput { contract_address }) => {
                self.stop_impersonating_account(contract_address).await
            }
            StarknetRequest::AutoImpersonate => self.auto_impersonate(true).await,
            StarknetRequest::StopAutoImpersonate => self.auto_impersonate(false).await,
        };

        if let (Err(err), Some(forwarder)) = (&starknet_resp, &self.origin_caller) {
            match err {
                // if a block or state is requested that was only added to origin after
                // forking happened, it will be normally returned; we don't extra-handle this case
                error::ApiError::BlockNotFound
                | error::ApiError::TransactionNotFound
                | error::ApiError::NoStateAtBlock { .. }
                | error::ApiError::ClassHashNotFound => {
                    // ClassHashNotFound can be thrown from starknet_getClass or
                    // starknet_deployAccount, but only starknet_getClass should be retried from
                    // here; starknet_deployAccount already fetches from origin internally. This is
                    // handled by (un)setting the `forwardable` flag

                    if forwardable {
                        return forwarder.call(&original_call).await;
                    }
                }
                _other_error => (),
            }
        }

        starknet_resp.to_rpc_result()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum StarknetRequest {
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
    #[serde(rename = "starknet_getTransactionByHash")]
    TransactionByHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionByBlockIdAndIndex")]
    TransactionByBlockAndIndex(BlockAndIndexInput),
    #[serde(rename = "starknet_getTransactionReceipt")]
    TransactionReceiptByTransactionHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionStatus")]
    TransactionStatusByHash(TransactionHashInput),
    #[serde(rename = "starknet_getClass")]
    ClassByHash(BlockAndClassHashInput),
    #[serde(rename = "starknet_getClassHashAt")]
    ClassHashAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getClassAt")]
    ClassAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getBlockTransactionCount")]
    BlockTransactionCount(BlockIdInput),
    #[serde(rename = "starknet_call")]
    Call(CallInput),
    #[serde(rename = "starknet_estimateFee")]
    EsimateFee(EstimateFeeInput),
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
    ImpersonateAccount(ContractAddressInput),
    #[serde(rename = "devnet_stopImpersonateAccount")]
    StopImpersonateAccount(ContractAddressInput),
    #[serde(rename = "devnet_autoImpersonate", with = "empty_params")]
    AutoImpersonate,
    #[serde(rename = "devnet_stopAutoImpersonate", with = "empty_params")]
    StopAutoImpersonate,
}

impl std::fmt::Display for StarknetRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StarknetRequest::SpecVersion => write!(f, "starknet_specVersion"),
            StarknetRequest::BlockWithTransactionHashes(_) => {
                write!(f, "starknet_getBlockWithTxHashes")
            }
            StarknetRequest::BlockWithFullTransactions(_) => write!(f, "starknet_getBlockWithTxs"),
            StarknetRequest::BlockWithReceipts(_) => write!(f, "starknet_getBlockWithReceipts"),
            StarknetRequest::StateUpdate(_) => write!(f, "starknet_getStateUpdate"),
            StarknetRequest::StorageAt(_) => write!(f, "starknet_getStorageAt"),
            StarknetRequest::TransactionByHash(_) => write!(f, "starknet_getTransactionByHash"),
            StarknetRequest::TransactionStatusByHash(_) => {
                write!(f, "starknet_getTransactionStatus")
            }
            StarknetRequest::TransactionByBlockAndIndex(_) => {
                write!(f, "starknet_getTransactionByBlockIdAndIndex")
            }
            StarknetRequest::TransactionReceiptByTransactionHash(_) => {
                write!(f, "starknet_getTransactionReceipt")
            }
            StarknetRequest::ClassByHash(_) => write!(f, "starknet_getClass"),
            StarknetRequest::ClassHashAtContractAddress(_) => write!(f, "starknet_getClassHashAt"),
            StarknetRequest::ClassAtContractAddress(_) => write!(f, "starknet_getClassAt"),
            StarknetRequest::BlockTransactionCount(_) => {
                write!(f, "starknet_getBlockTransactionCount")
            }
            StarknetRequest::Call(_) => write!(f, "starknet_call"),
            StarknetRequest::EsimateFee(_) => write!(f, "starknet_estimateFee"),
            StarknetRequest::BlockNumber => write!(f, "starknet_blockNumber"),
            StarknetRequest::BlockHashAndNumber => write!(f, "starknet_blockHashAndNumber"),
            StarknetRequest::ChainId => write!(f, "starknet_chainId"),
            StarknetRequest::Syncing => write!(f, "starknet_syncing"),
            StarknetRequest::Events(_) => write!(f, "starknet_getEvents"),
            StarknetRequest::ContractNonce(_) => write!(f, "starknet_getNonce"),
            StarknetRequest::AddDeclareTransaction(_) => {
                write!(f, "starknet_addDeclareTransaction")
            }
            StarknetRequest::AddDeployAccountTransaction(_) => {
                write!(f, "starknet_addDeployAccountTransaction")
            }
            StarknetRequest::AddInvokeTransaction(_) => write!(f, "starknet_addInvokeTransaction"),
            StarknetRequest::EstimateMessageFee(_) => write!(f, "starknet_estimateMessageFee"),
            StarknetRequest::SimulateTransactions(_) => write!(f, "starknet_simulateTransactions"),
            StarknetRequest::TraceTransaction(_) => write!(f, "starknet_traceTransaction"),
            StarknetRequest::BlockTransactionTraces(_) => {
                write!(f, "starknet_traceBlockTransactions")
            }
            StarknetRequest::ImpersonateAccount(_) => write!(f, "devnet_impersonateAccount"),
            StarknetRequest::StopImpersonateAccount(_) => {
                write!(f, "devnet_stopImpersonateAccount")
            }
            StarknetRequest::AutoImpersonate => write!(f, "devnet_autoImpersonate"),
            StarknetRequest::StopAutoImpersonate => write!(f, "devnet_stopAutoImpersonate"),
        }
    }
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(untagged)]
pub enum StarknetResponse {
    Block(Block),
    StateUpdate(StateUpdate),
    Felt(Felt),
    Transaction(TransactionWithHash),
    TransactionReceiptByTransactionHash(Box<TransactionReceipt>),
    TransactionStatusByHash(TransactionStatusOutput),
    ContractClass(CodegenContractClass),
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
    AddInvokeTransaction(InvokeTransactionOutput),
    EstimateMessageFee(FeeEstimateWrapper),
    SimulateTransactions(Vec<SimulatedTransaction>),
    TraceTransaction(TransactionTrace),
    BlockTransactionTraces(Vec<BlockTransactionTrace>),
    Empty,
}

#[cfg(test)]
mod requests_tests {

    use serde_json::json;
    use starknet_types::felt::Felt;

    use super::StarknetRequest;
    use crate::test_utils::exported_test_utils::assert_contains;

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

        let request = serde_json::from_str::<StarknetRequest>(json_str).unwrap();

        match request {
            StarknetRequest::TransactionByHash(input) => {
                assert!(input.transaction_hash == Felt::from_prefixed_hex_str("0x134134").unwrap());
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
            "Missing prefix 0x in 134134",
        );
        // Errored json, hex is longer than 64 chars
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x004134134134134134134134134134134134134134134134134134134134134134"}}"#,
            "Bad input - expected #bytes: 32",
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

        assert_deserialization_fails(json_str.replace("0x", "").as_str(), "Missing prefix 0x in");
    }

    #[test]
    fn deserialize_get_class_request() {
        let json_str = r#"{"method":"starknet_getClass","params":{"block_id":"latest","class_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0x", "").as_str(), "Missing prefix 0x");
    }

    #[test]
    fn deserialize_get_class_hash_at_request() {
        let json_str = r#"{"method":"starknet_getClassHashAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0x", "").as_str(), "Missing prefix 0x");
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
            "Missing prefix 0x",
        );
        assert_deserialization_fails(
            json_str
                .replace(
                    r#""entry_point_selector":"0x134134""#,
                    r#""entry_point_selector":"134134""#,
                )
                .as_str(),
            "Missing prefix 0x",
        );
        assert_deserialization_fails(
            json_str.replace(r#""calldata":["0x134134"]"#, r#""calldata":["123"]"#).as_str(),
            "Missing prefix 0x",
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

    fn assert_deserialization_succeeds(json_str: &str) {
        serde_json::from_str::<StarknetRequest>(json_str).unwrap();
    }

    fn assert_deserialization_fails(json_str: &str, expected_msg: &str) {
        match serde_json::from_str::<StarknetRequest>(json_str) {
            Err(err) => assert_contains(&err.to_string(), expected_msg),
            other => panic!("Invalid result: {other:?}"),
        }
    }
}

#[cfg(test)]
mod response_tests {
    use crate::api::json_rpc::error::StrictRpcResult;
    use crate::api::json_rpc::{StarknetResponse, ToRpcResponseResult};

    #[test]
    fn serializing_starknet_response_empty_variant_have_to_produce_empty_json_object_when_converted_to_rpc_result()
     {
        assert_eq!(
            r#"{"result":{}}"#,
            serde_json::to_string(&StrictRpcResult::Ok(StarknetResponse::Empty).to_rpc_result())
                .unwrap()
        );
    }
}
