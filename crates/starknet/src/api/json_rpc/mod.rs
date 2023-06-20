mod error;
mod models;
mod serde_helpers;

use crate::api::json_rpc::models::request_response::{
    BlockAndClassHashInput, BlockAndContractAddressInput, BlockAndIndexInput, CallInput,
    EstimateFeeInput, EventsInput, GetStorageInput, TransactionHashInput,
};

use self::error::RpcResult;
use self::models::block::Block;
use self::models::contract_class::ContractClass;
use self::models::request_response::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};
use self::models::state::ThinStateDiff;
use self::models::transaction::{
    ClassHashHex, EventFilter, EventsChunk, FunctionCall, Transaction, TransactionHashHex,
    TransactionReceipt, TransactionWithType,
};
use self::models::{BlockId, ContractAddressHex, FeltHex, PatriciaKeyHex};
use super::Api;

use serde::Deserialize;
use server::rpc_core::response::ResponseResult;
use server::rpc_handler::RpcHandler;
use starknet_types::starknet_api::block::BlockNumber;
use tracing::{info, trace};

use self::error::ToRpcResponseResult;
use self::serde_helpers::empty_params;

#[async_trait::async_trait]
impl RpcHandler for JsonRpcHandler {
    type Request = StarknetRequest;

    async fn on_request(&self, request: Self::Request) -> ResponseResult {
        info!(target: "rpc", "received method in on_request");
        self.execute(request).await
    }
}

#[derive(Clone)]
pub struct JsonRpcHandler {
    pub api: Api,
}

impl JsonRpcHandler {
    async fn execute(&self, request: StarknetRequest) -> ResponseResult {
        trace!(target: "JsonRpcHandler::execute", "executing starknet request");

        match request {
            StarknetRequest::StarknetServerVersion => self.version().to_rpc_result(),
            StarknetRequest::BlockWithTransactionHashes(block) => self
                .get_block_with_tx_hashes(block.block_id)
                .await
                .to_rpc_result(),
            StarknetRequest::BlockWithFullTransactions(block) => self
                .get_block_with_full_txs(block.block_id)
                .await
                .to_rpc_result(),
            StarknetRequest::StateUpdate(block) => {
                self.get_state_update(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::StorageAt(GetStorageInput {
                contract_address,
                key,
                block_id,
            }) => self
                .get_storage_at(contract_address, key, block_id)
                .await
                .to_rpc_result(),
            StarknetRequest::TransactionByHash(TransactionHashInput { transaction_hash }) => self
                .get_transaction_by_hash(transaction_hash)
                .await
                .to_rpc_result(),
            StarknetRequest::TransactionByBlockAndIndex(BlockAndIndexInput { block_id, index }) => {
                self.get_transaction_by_block_id_and_index(block_id, index)
                    .await
                    .to_rpc_result()
            }
            StarknetRequest::Test(_) => todo!(),
            StarknetRequest::TransactionReceiptByTransactionHash(TransactionHashInput {
                transaction_hash,
            }) => self
                .get_transaction_receipt_by_hash(transaction_hash)
                .await
                .to_rpc_result(),
            StarknetRequest::ClassByHash(BlockAndClassHashInput {
                block_id,
                class_hash,
            }) => self.get_class(block_id, class_hash).await.to_rpc_result(),
            StarknetRequest::ClassHashAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self
                .get_class_hash_at(block_id, contract_address)
                .await
                .to_rpc_result(),
            StarknetRequest::ClassAtContractAddress(BlockAndContractAddressInput {
                block_id,
                contract_address,
            }) => self
                .get_class_at(block_id, contract_address)
                .await
                .to_rpc_result(),
            StarknetRequest::BlockTransactionCount(block) => self
                .get_block_txs_count(block.block_id)
                .await
                .to_rpc_result(),
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
            }) => self
                .get_nonce(block_id, contract_address)
                .await
                .to_rpc_result(),
        }
    }
}

impl JsonRpcHandler {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn get_block_with_tx_hashes(&self, block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn get_block_with_full_txs(&self, block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn get_state_update(&self, block_id: BlockId) -> RpcResult<ThinStateDiff> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn get_storage_at(
        &self,
        contract_address: ContractAddressHex,
        key: PatriciaKeyHex,
        block_id: BlockId,
    ) -> RpcResult<PatriciaKeyHex> {
        Err(error::ApiError::ContractNotFound)
    }

    async fn get_transaction_by_hash(
        &self,
        transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::TransactionNotFound(transaction_hash))
    }

    async fn get_transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        index: BlockNumber,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::InvalidTransactionIndexInBlock(
            index, block_id,
        ))
    }

    async fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionReceipt> {
        Err(error::ApiError::TransactionNotFound(transaction_hash))
    }

    async fn get_class(
        &self,
        block_id: BlockId,
        class_hash: ClassHashHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ClassHashNotFound(class_hash))
    }

    async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ClassHashHex> {
        Err(error::ApiError::ContractNotFound)
    }

    async fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ContractNotFound)
    }

    async fn get_block_txs_count(&self, block_id: BlockId) -> RpcResult<BlockNumber> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn call(&self, block_id: BlockId, request: FunctionCall) -> RpcResult<Vec<FeltHex>> {
        Err(error::ApiError::ContractError)
    }

    async fn estimate_fee(
        &self,
        block_id: BlockId,
        request: Vec<Transaction>,
    ) -> RpcResult<Vec<EstimateFeeOutput>> {
        Err(error::ApiError::ContractError)
    }

    async fn block_number(&self) -> RpcResult<BlockNumber> {
        Err(error::ApiError::NoBlocks)
    }

    async fn block_hash_and_number(&self) -> RpcResult<BlockHashAndNumberOutput> {
        Err(error::ApiError::NoBlocks)
    }

    fn chain_id(&self) -> RpcResult<String> {
        Ok("0xAA".to_string())
    }

    async fn pending_transactions(&self) -> RpcResult<Vec<Transaction>> {
        Ok(vec![])
    }

    async fn syncing(&self) -> RpcResult<SyncingOutput> {
        Ok(SyncingOutput::False(false))
    }

    async fn get_events(&self, filter: EventFilter) -> RpcResult<EventsChunk> {
        Err(error::ApiError::InvalidContinuationToken)
    }

    async fn get_nonce(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<FeltHex> {
        Err(error::ApiError::BlockNotFound)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Eq)]
#[serde(tag = "method", content = "params")]
pub enum StarknetRequest {
    #[serde(rename = "starknet_clientVersion", with = "empty_params")]
    StarknetServerVersion,
    #[serde(rename = "starknet_getBlockWithTxHashes")]
    BlockWithTransactionHashes(models::request_response::BlockIdInput),
    #[serde(rename = "starknet_getBlockWithTxs")]
    BlockWithFullTransactions(models::request_response::BlockIdInput),
    #[serde(rename = "starknet_getStateUpdate")]
    StateUpdate(models::request_response::BlockIdInput),
    #[serde(rename = "starknet_getStorageAt")]
    StorageAt(models::request_response::GetStorageInput),
    #[serde(rename = "starknet_getTransactionByHash")]
    TransactionByHash(models::request_response::TransactionHashInput),
    #[serde(rename = "starknet_getTransactionByBlockIdAndIndex")]
    TransactionByBlockAndIndex(models::request_response::BlockAndIndexInput),
    #[serde(rename = "starknet_getTransactionReceipt")]
    TransactionReceiptByTransactionHash(models::request_response::TransactionHashInput),
    #[serde(rename = "starknet_getClass")]
    ClassByHash(models::request_response::BlockAndClassHashInput),
    #[serde(rename = "starknet_getClassHashAt")]
    ClassHashAtContractAddress(models::request_response::BlockAndContractAddressInput),
    #[serde(rename = "starknet_getClassAt")]
    ClassAtContractAddress(models::request_response::BlockAndContractAddressInput),
    #[serde(rename = "starknet_getBlockTransactionsCount")]
    BlockTransactionCount(models::request_response::BlockIdInput),
    #[serde(rename = "starknet_call")]
    Call(models::request_response::CallInput),
    #[serde(rename = "starknet_estimateFee")]
    EsimateFee(models::request_response::EstimateFeeInput),
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
    Events(models::request_response::EventsInput),
    #[serde(rename = "starknet_getNonce")]
    ContractNonce(models::request_response::BlockAndContractAddressInput),
    #[serde(rename = "test_string")]
    Test(String),
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::StarknetRequest;

    #[test]
    fn deserialize_to_no_params_web3_client_version() {
        let json_obj = r#"{"method":"starknet_clientVersion","params":[]}"#;
        let call: Value = serde_json::from_str(json_obj).unwrap();

        let x: StarknetRequest = serde_json::from_value(call).unwrap();
        println!("{:?}", x);

        assert_deserialize_to_expected_enum_type(json_obj, StarknetRequest::StarknetServerVersion);
    }

    #[test]
    fn deserialize_to_params_test_string() {
        let json_obj = r#"{"method":"test_string","params":"aa"}"#;
        let call: Value = serde_json::from_str(json_obj).unwrap();

        let x: StarknetRequest = serde_json::from_value(call).unwrap();
        assert_eq!(x, StarknetRequest::Test("aa".to_string()));

        assert_deserialize_to_expected_enum_type(json_obj, StarknetRequest::Test("aa".to_string()));
    }

    #[test]
    fn deserialize_get_transaction_by_hash_request() {
        let json_str = r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x134134"}}"#;
        let json_str = r#"{"method":"starknet_getTransactionByHash","params":"0x134134"}"#;
        let call: Value = serde_json::from_str(json_str).unwrap();

        let x = serde_json::from_value::<StarknetRequest>(call);
        println!("{:?}", x);
    }

    fn assert_deserialize_to_expected_enum_type(
        json_str: &str,
        expected_enum_type: StarknetRequest,
    ) {
        let generated: StarknetRequest = serde_json::from_str(json_str).unwrap();

        assert!(matches!(generated, expected_enum_type));
    }
}
