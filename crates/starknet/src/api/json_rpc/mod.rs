mod error;
mod models;
mod serde_helpers;

use crate::api::json_rpc::models::request_input::{
    BlockAndIndexInput, GetStorageInput, TransactionHashInput,
};

use self::error::RpcResult;
use self::models::block::Block;
use self::models::state::ThinStateDiff;
use self::models::transaction::{TransactionHashHex, TransactionWithType};
use self::models::{BlockId, ContractAddressHex, PatriciaKeyHex};
use super::Api;

use serde::Deserialize;
use server::rpc_core::response::ResponseResult;
use server::rpc_handler::RpcHandler;
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
                .block_with_tx_hashes(block.block_id)
                .await
                .to_rpc_result(),
            StarknetRequest::BlockWithFullTransactions(block) => self
                .block_with_full_txs(block.block_id)
                .await
                .to_rpc_result(),
            StarknetRequest::StateUpdate(block) => {
                self.state_update(block.block_id).await.to_rpc_result()
            }
            StarknetRequest::StorageAt(GetStorageInput {
                contract_address,
                key,
                block_id,
            }) => self
                .storage_at(contract_address, key, block_id)
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
        }
    }
}

impl JsonRpcHandler {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn block_with_tx_hashes(&self, block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn block_with_full_txs(&self, block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn state_update(&self, block_id: BlockId) -> RpcResult<ThinStateDiff> {
        Err(error::ApiError::BlockNotFound)
    }

    async fn storage_at(
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
        index: usize,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::InvalidTransactionIndexInBlock(
            index, block_id,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Eq)]
#[serde(tag = "method", content = "params")]
pub enum StarknetRequest {
    #[serde(rename = "starknet_clientVersion", with = "empty_params")]
    StarknetServerVersion,
    #[serde(rename = "starknet_getBlockWithTxHashes")]
    BlockWithTransactionHashes(models::request_input::BlockIdInput),
    #[serde(rename = "starknet_getBlockWithTxs")]
    BlockWithFullTransactions(models::request_input::BlockIdInput),
    #[serde(rename = "starknet_getStateUpdate")]
    StateUpdate(models::request_input::BlockIdInput),
    #[serde(rename = "starknet_getStorageAt")]
    StorageAt(models::request_input::GetStorageInput),
    #[serde(rename = "starknet_getTransactionByHash")]
    TransactionByHash(models::request_input::TransactionHashInput),
    #[serde(rename = "starknet_getTransactionByBlockIdAndIndex")]
    TransactionByBlockAndIndex(models::request_input::BlockAndIndexInput),
    #[serde(rename = "starknet_getTransactionReceipt")]
    TransactionReceiptByTransactionHash(models::request_input::TransactionHashInput),
    #[serde(rename = "starknet_getClass")]
    ClassByHash(models::request_input::BlockAndClassHashInput),
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
