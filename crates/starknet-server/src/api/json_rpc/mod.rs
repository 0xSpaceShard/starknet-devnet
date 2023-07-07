mod endpoints;
mod error;
mod models;

use models::{
    BlockAndClassHashInput, BlockAndContractAddressInput, BlockAndIndexInput, CallInput,
    EstimateFeeInput, EventsInput, GetStorageInput, TransactionHashInput,
};
use serde::{Deserialize, Serialize};
use server::rpc_core::error::RpcError;
use server::rpc_core::response::ResponseResult;
use server::rpc_handler::RpcHandler;
use tracing::{error, info, trace};

use self::error::ApiError;
use self::models::BlockIdInput;
use super::Api;
use crate::api::serde_helpers::empty_params;

pub(crate) type RpcResult<T> = std::result::Result<T, ApiError>;

/// Helper trait to easily convert results to rpc results
pub(crate) trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

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

impl<T: Serialize> ToRpcResponseResult for RpcResult<T> {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(data) => to_rpc_result(data),
            Err(err) => match err {
                ApiError::RpcError(rpc_error) => rpc_error,
                err @ ApiError::BlockNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(24),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ContractNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(20),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::TransactionNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(25),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::InvalidTransactionIndexInBlock => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(27),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ClassHashNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(28),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ContractError => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(40),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::NoBlocks => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(32),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::RequestPageSizeTooBig => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(31),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::InvalidContinuationToken => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(33),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::TooManyKeysInFilter => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(34),
                    message: err.to_string().into(),
                    data: None,
                },
            }
            .into(),
        }
    }
}

#[async_trait::async_trait]
impl RpcHandler for JsonRpcHandler {
    type Request = StarknetRequest;

    async fn on_request(&self, request: Self::Request) -> ResponseResult {
        info!(target: "rpc", "received method in on_request");
        self.execute(request).await
    }
}

/// This object will be used as a shared state between HTTP calls.
/// Is simillar to the HttpApiHandler but is with extended functionality and is used for JSON-RPC
/// methods
#[derive(Clone)]
pub struct JsonRpcHandler {
    pub api: Api,
}

impl JsonRpcHandler {
    /// The method matches the request to the corresponding enum variant and executes the request
    async fn execute(&self, request: StarknetRequest) -> ResponseResult {
        trace!(target: "JsonRpcHandler::execute", "executing starknet request");

        match request {
            StarknetRequest::BlockWithTransactionHashes(block) => {
                self.get_block_with_tx_hashes(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::BlockWithFullTransactions(block) => {
                self.get_block_with_full_txs(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::StateUpdate(block) => {
                self.get_state_update(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::StorageAt(GetStorageInput { contract_address, key, block_id }) => {
                self.get_storage_at(contract_address, key, block_id).await.to_rpc_result()
            }
            StarknetRequest::TransactionByHash(TransactionHashInput { transaction_hash }) => {
                self.get_transaction_by_hash(transaction_hash).await.to_rpc_result()
            }
            StarknetRequest::TransactionByBlockAndIndex(BlockAndIndexInput { block_id, index }) => {
                self.get_transaction_by_block_id_and_index(block_id, index).await.to_rpc_result()
            }
            StarknetRequest::TransactionReceiptByTransactionHash(TransactionHashInput {
                transaction_hash,
            }) => self.get_transaction_receipt_by_hash(transaction_hash).await.to_rpc_result(),
            StarknetRequest::ClassByHash(BlockAndClassHashInput { block_id, class_hash }) => {
                self.get_class(block_id, class_hash).await.to_rpc_result()
            }
            StarknetRequest::ClassHashAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_hash_at(block_id, contract_address).await.to_rpc_result(),
            StarknetRequest::ClassAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_class_at(block_id, contract_address).await.to_rpc_result(),
            StarknetRequest::BlockTransactionCount(block) => {
                self.get_block_txs_count(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::Call(CallInput { request, block_id }) => {
                self.call(block_id, request).await.to_rpc_result()
            }
            StarknetRequest::EsimateFee(EstimateFeeInput { request, block_id }) => {
                self.estimate_fee(block_id, request).await.to_rpc_result()
            }
            StarknetRequest::BlockNumber => self.block_number().await.to_rpc_result(),
            StarknetRequest::BlockHashAndNumber => {
                self.block_hash_and_number().await.to_rpc_result()
            }
            StarknetRequest::ChainId => self.chain_id().to_rpc_result(),
            StarknetRequest::PendingTransactions => {
                self.pending_transactions().await.to_rpc_result()
            }
            StarknetRequest::Syncing => self.syncing().await.to_rpc_result(),
            StarknetRequest::Events(EventsInput { filter }) => {
                self.get_events(filter).await.to_rpc_result()
            }
            StarknetRequest::ContractNonce(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self.get_nonce(block_id, contract_address).await.to_rpc_result(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Eq)]
#[serde(tag = "method", content = "params")]
pub enum StarknetRequest {
    #[serde(rename = "starknet_getBlockWithTxHashes")]
    BlockWithTransactionHashes(BlockIdInput),
    #[serde(rename = "starknet_getBlockWithTxs")]
    BlockWithFullTransactions(BlockIdInput),
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
    #[serde(rename = "starknet_pendingTransactions", with = "empty_params")]
    PendingTransactions,
    #[serde(rename = "starknet_syncing", with = "empty_params")]
    Syncing,
    #[serde(rename = "starknet_getEvents")]
    Events(EventsInput),
    #[serde(rename = "starknet_getNonce")]
    ContractNonce(BlockAndContractAddressInput),
}

#[cfg(test)]
mod requests_tests {
    use starknet_types::felt::Felt;

    use super::StarknetRequest;

    #[test]
    fn deserialize_get_block_with_transaction_hashes_request() {
        let json_str =
            r#"{"method":"starknet_getBlockWithTxHashes","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(&json_str.replace("latest", "0x134134"));
    }

    #[test]
    fn deserialize_get_block_with_transactions_request() {
        let json_str = r#"{"method":"starknet_getBlockWithTxs","params":{"block_id":"pending"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("pending", "0x134134").as_str());
    }

    #[test]
    fn deserialize_get_state_update_request() {
        let json_str = r#"{"method":"starknet_getStateUpdate","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(&json_str.replace("latest", "0x134134"));
    }

    #[test]
    fn deserialize_get_storage_at_request() {
        let json_str = r#"{"method":"starknet_getStorageAt","params":{"contract_address":"0x134134","key":"0x134134","block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(&json_str.replace("0x134134", "134134"));
    }

    #[test]
    fn deserialize_get_transaction_by_hash_request() {
        let json_str = r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x134134"}}"#;

        let request = serde_json::from_str::<StarknetRequest>(json_str).unwrap();

        match request {
            StarknetRequest::TransactionByHash(input) => {
                assert!(
                    input.transaction_hash.0 == Felt::from_prefixed_hex_str("0x134134").unwrap()
                );
            }
            _ => panic!("Wrong request type"),
        }

        // Errored json, there is no object just string is passed
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":"0x134134"}"#,
        );
        // Errored json, hash is not prefixed with 0x
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"134134"}}"#,
        );
        // Errored json, hex is longer than 64 chars
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x004134134134134134134134134134134134134134134134134134134134134134"}}"#,
        );
    }

    #[test]
    fn deserialize_get_transaction_by_block_and_index_request() {
        let json_str = r#"{"method":"starknet_getTransactionByBlockIdAndIndex","params":{"block_id":"latest","index":0}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace('0', "0x134134").as_str());
    }

    #[test]
    fn deserialize_get_transaction_receipt_request() {
        let json_str = r#"{"method":"starknet_getTransactionReceipt","params":{"transaction_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0xAAABB", "134134").as_str());
    }

    #[test]
    fn deserialize_get_class_request() {
        let json_str = r#"{"method":"starknet_getClass","params":{"block_id":"latest","class_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0xAAABB", "134134").as_str());
    }

    #[test]
    fn deserialize_get_class_hash_at_request() {
        let json_str = r#"{"method":"starknet_getClassHashAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0xAAABB", "134134").as_str());
    }

    #[test]
    fn deserialize_get_class_at_request() {
        let json_str = r#"{"method":"starknet_getClassAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0xAAABB", "134134").as_str());
    }

    #[test]
    fn deserialize_get_block_transaction_count_request() {
        let json_str =
            r#"{"method":"starknet_getBlockTransactionCount","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("latest", "0x134134").as_str());
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

        assert_deserialization_fails(json_str.replace("starknet_call", "starknet_Call").as_str());

        assert_deserialization_fails(json_str.replace("0xAAABB", "134134").as_str());
    }

    #[test]
    fn deserialize_estimate_fee_request() {
        let json_str = r#"{
            "method":"starknet_estimateFee",
            "params":{
                "block_id":"latest",
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

        assert_deserialization_fails(json_str.replace("estimateFee", "estimate_fee").as_str());
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

        assert_deserialization_fails(json_str.replace(r#""chunk_size": 1,"#, "").as_str());
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
        assert_deserialization_fails(json_str.replace(r#""block_id":"latest","#, "").as_str());
    }

    fn assert_deserialization_succeeds(json_str: &str) {
        serde_json::from_str::<StarknetRequest>(json_str).unwrap();
    }

    fn assert_deserialization_fails(json_str: &str) {
        assert!(serde_json::from_str::<StarknetRequest>(json_str).is_err());
    }
}
