use crate::api::models::{block::Block, state::ThinStateDiff, BlockId};
use crate::api::models::contract_class::ContractClass;
use crate::api::models::transaction::{
    BroadcastedTransactionWithType, ClassHashHex, EventFilter, EventsChunk, FunctionCall,
    Transaction, TransactionHashHex, TransactionReceipt, TransactionWithType,
};
use crate::api::models::{ContractAddressHex, FeltHex, PatriciaKeyHex};

use starknet_types::starknet_api::block::BlockNumber;

use super::RpcResult;
use super::models::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};

use super::{
    error::{self},
    JsonRpcHandler,
};

/// here is definiton and stub implementation of all JSON-RPC endpoints
impl JsonRpcHandler {
    pub(crate) async fn get_block_with_tx_hashes(&self, _block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    pub(crate) async fn get_block_with_full_txs(&self, _block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    pub(crate) async fn get_state_update(&self, _block_id: BlockId) -> RpcResult<ThinStateDiff> {
        Err(error::ApiError::BlockNotFound)
    }

    pub(crate) async fn get_storage_at(
        &self,
        _contract_address: ContractAddressHex,
        _key: PatriciaKeyHex,
        _block_id: BlockId,
    ) -> RpcResult<PatriciaKeyHex> {
        Err(error::ApiError::ContractNotFound)
    }

    pub(crate) async fn get_transaction_by_hash(
        &self,
        _transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::TransactionNotFound)
    }

    pub(crate) async fn get_transaction_by_block_id_and_index(
        &self,
        _block_id: BlockId,
        _index: BlockNumber,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::InvalidTransactionIndexInBlock)
    }

    pub(crate) async fn get_transaction_receipt_by_hash(
        &self,
        _transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionReceipt> {
        Err(error::ApiError::TransactionNotFound)
    }

    pub(crate) async fn get_class(
        &self,
        _block_id: BlockId,
        _class_hash: ClassHashHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ClassHashNotFound)
    }

    pub(crate) async fn get_class_hash_at(
        &self,
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<ClassHashHex> {
        Err(error::ApiError::ContractNotFound)
    }

    pub(crate) async fn get_class_at(
        &self,
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ContractNotFound)
    }

    pub(crate) async fn get_block_txs_count(&self, _block_id: BlockId) -> RpcResult<BlockNumber> {
        Err(error::ApiError::BlockNotFound)
    }

    pub(crate) async fn call(
        &self,
        _block_id: BlockId,
        _request: FunctionCall,
    ) -> RpcResult<Vec<FeltHex>> {
        Err(error::ApiError::ContractError)
    }

    pub(crate) async fn estimate_fee(
        &self,
        _block_id: BlockId,
        _request: Vec<BroadcastedTransactionWithType>,
    ) -> RpcResult<Vec<EstimateFeeOutput>> {
        Err(error::ApiError::ContractError)
    }

    pub(crate) async fn block_number(&self) -> RpcResult<BlockNumber> {
        Err(error::ApiError::NoBlocks)
    }

    pub(crate) async fn block_hash_and_number(&self) -> RpcResult<BlockHashAndNumberOutput> {
        Err(error::ApiError::NoBlocks)
    }

    pub(crate) fn chain_id(&self) -> RpcResult<String> {
        // DEVNET
        Ok("0x4445564e4554".to_string())
    }

    pub(crate) async fn pending_transactions(&self) -> RpcResult<Vec<Transaction>> {
        Ok(vec![])
    }

    pub(crate) async fn syncing(&self) -> RpcResult<SyncingOutput> {
        Ok(SyncingOutput::False(false))
    }

    pub(crate) async fn get_events(&self, _filter: EventFilter) -> RpcResult<EventsChunk> {
        Err(error::ApiError::InvalidContinuationToken)
    }

    pub(crate) async fn get_nonce(
        &self,
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<FeltHex> {
        Err(error::ApiError::BlockNotFound)
    }
}
