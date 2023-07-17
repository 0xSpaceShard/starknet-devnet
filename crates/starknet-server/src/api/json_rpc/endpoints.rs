use starknet_types::felt::Felt;
use starknet_types::starknet_api::block::BlockNumber;

use crate::api::{
    json_rpc::{
        error::{self},
        JsonRpcHandler, RpcResult,
    },
    models::{
        block::Block,
        contract_class::ContractClass,
        state::ThinStateDiff,
        transaction::{
            BroadcastedTransactionWithType, ClassHashHex, EventFilter, EventsChunk, FunctionCall,
            Transaction, TransactionHashHex, TransactionReceipt, TransactionWithType,
        },
        BlockHashAndNumberOutput, BlockId, ContractAddressHex, EstimateFeeOutput, FeltHex,
        PatriciaKeyHex, SyncingOutput,
    },
};

/// here are the definitions and stub implementations of all JSON-RPC read endpoints
impl JsonRpcHandler {
    /// starknet_getBlockWithTxHashes
    pub(crate) async fn get_block_with_tx_hashes(&self, _block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    /// starknet_getBlockWithTxs
    pub(crate) async fn get_block_with_full_txs(&self, _block_id: BlockId) -> RpcResult<Block> {
        Err(error::ApiError::BlockNotFound)
    }

    /// starknet_getStateUpdate
    pub(crate) async fn get_state_update(&self, _block_id: BlockId) -> RpcResult<ThinStateDiff> {
        Err(error::ApiError::BlockNotFound)
    }

    /// starknet_getStorageAt
    pub(crate) async fn get_storage_at(
        &self,
        _contract_address: ContractAddressHex,
        _key: PatriciaKeyHex,
        _block_id: BlockId,
    ) -> RpcResult<PatriciaKeyHex> {
        Err(error::ApiError::ContractNotFound)
    }

    /// starknet_getTransactionByHash
    pub(crate) async fn get_transaction_by_hash(
        &self,
        _transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::TransactionNotFound)
    }

    /// starknet_getTransactionByBlockIdAndIndex
    pub(crate) async fn get_transaction_by_block_id_and_index(
        &self,
        _block_id: BlockId,
        _index: BlockNumber,
    ) -> RpcResult<TransactionWithType> {
        Err(error::ApiError::InvalidTransactionIndexInBlock)
    }

    /// starknet_getTransactionReceipt
    pub(crate) async fn get_transaction_receipt_by_hash(
        &self,
        _transaction_hash: TransactionHashHex,
    ) -> RpcResult<TransactionReceipt> {
        Err(error::ApiError::TransactionNotFound)
    }

    /// starknet_getClass
    pub(crate) async fn get_class(
        &self,
        block_id: BlockId,
        class_hash: ClassHashHex,
    ) -> RpcResult<ContractClass> {
        Ok(self.api.starknet.read().await.get_class(block_id, class_hash).unwrap())
    }

    /// starknet_getClassHashAt
    pub(crate) async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ClassHashHex> {
        let parsed_address = contract_address.0.try_into()?;

        let starknet = self.api.starknet.read().await;
        let state = starknet.get_state_reader_at(&block_id.into())?;
        match state.address_to_class_hash.get(&parsed_address) {
            Some(class_hash) => Ok(FeltHex(Felt::from(*class_hash))),
            None => Err(error::ApiError::ContractNotFound),
        }
    }

    /// starknet_getClassAt
    pub(crate) async fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ContractClass> {
        let parsed_address = contract_address.0.try_into().map_err(|_| {
            error::ApiError::RpcError(RpcError::invalid_params(format!(
                "Invalid contract_address: {:?}",
                contract_address.0
            )))
        })?;

        let starknet = self.api.starknet.read().await;
        let state = starknet
            .get_state_reader_at(&block_id.into())
            .map_err(|_| error::ApiError::BlockNotFound)?;
        match state.address_to_class_hash.get(&parsed_address) {
            Some(class_hash) => Ok(FeltHex(Felt::from(*class_hash))),
            None => Err(error::ApiError::ContractNotFound),
        }
    }

    /// starknet_getBlockTransactionCount
    pub(crate) async fn get_block_txs_count(&self, _block_id: BlockId) -> RpcResult<BlockNumber> {
        Err(error::ApiError::BlockNotFound)
    }

    /// starknet_call
    pub(crate) async fn call(
        &self,
        _block_id: BlockId,
        _request: FunctionCall,
    ) -> RpcResult<Vec<FeltHex>> {
        Err(error::ApiError::ContractError)
    }

    /// starknet_estimateFee
    pub(crate) async fn estimate_fee(
        &self,
        _block_id: BlockId,
        _request: Vec<BroadcastedTransactionWithType>,
    ) -> RpcResult<Vec<EstimateFeeOutput>> {
        Err(error::ApiError::ContractError)
    }

    /// starknet_blockNumber
    pub(crate) async fn block_number(&self) -> RpcResult<BlockNumber> {
        Err(error::ApiError::NoBlocks)
    }

    /// starknet_blockHashAndNumber
    pub(crate) async fn block_hash_and_number(&self) -> RpcResult<BlockHashAndNumberOutput> {
        Err(error::ApiError::NoBlocks)
    }

    /// starknet_chainId
    pub(crate) fn chain_id(&self) -> RpcResult<String> {
        Ok("TESTNET".to_string())
    }

    /// starknet_pendingTransactions
    pub(crate) async fn pending_transactions(&self) -> RpcResult<Vec<Transaction>> {
        Ok(vec![])
    }

    /// starknet_syncing
    pub(crate) async fn syncing(&self) -> RpcResult<SyncingOutput> {
        Ok(SyncingOutput::False(false))
    }

    /// starknet_getEvents
    pub(crate) async fn get_events(&self, _filter: EventFilter) -> RpcResult<EventsChunk> {
        Err(error::ApiError::InvalidContinuationToken)
    }

    /// starknet_getNonce
    pub(crate) async fn get_nonce(
        &self,
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<FeltHex> {
        Err(error::ApiError::BlockNotFound)
    }
}
