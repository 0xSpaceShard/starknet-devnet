use starknet_core::error::Error;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_types::starknet_api::block::BlockNumber;

use super::error::{self, ApiError};
use super::models::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};
use super::{JsonRpcHandler, RpcResult};
use crate::api::models::block::Block;
use crate::api::models::contract_class::ContractClass;
use crate::api::models::state::{
    ClassHashes, ContractNonce, DeployedContract, StateUpdate, StorageDiff, StorageEntry,
    ThinStateDiff,
};
use crate::api::models::transaction::{
    BroadcastedTransactionWithType, ClassHashHex, EventFilter, EventsChunk, FunctionCall,
    Transaction, TransactionHashHex, TransactionReceipt, TransactionWithType,
};
use crate::api::models::{BlockId, ContractAddressHex, FeltHex, PatriciaKeyHex};

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
    pub(crate) async fn get_state_update(&self, block_id: BlockId) -> RpcResult<StateUpdate> {
        let state_update =
            self.api.starknet.read().await.block_state_update(block_id.into()).map_err(|err| {
                match err {
                    Error::NoBlock => ApiError::BlockNotFound,
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                }
            })?;

        let state_diff = ThinStateDiff {
            deployed_contracts: state_update
                .deployed_contracts
                .into_iter()
                .map(|(address, class_hash)| DeployedContract {
                    address: ContractAddressHex(address),
                    class_hash: FeltHex(class_hash),
                })
                .collect(),
            declared_classes: state_update
                .declared_classes
                .into_iter()
                .map(|(class_hash, compiled_class_hash)| ClassHashes {
                    class_hash: FeltHex(class_hash),
                    compiled_class_hash: FeltHex(compiled_class_hash),
                })
                .collect(),
            deprecated_declared_classes: state_update
                .cairo_0_declared_classes
                .into_iter()
                .map(FeltHex)
                .collect(),
            nonces: state_update
                .nonces
                .into_iter()
                .map(|(address, nonce)| ContractNonce {
                    contract_address: ContractAddressHex(address),
                    nonce: FeltHex(nonce),
                })
                .collect(),
            storage_diffs: state_update
                .storage_updates
                .into_iter()
                .map(|(contract_address, updates)| StorageDiff {
                    address: ContractAddressHex(contract_address),
                    storage_entries: updates
                        .into_iter()
                        .map(|(key, value)| StorageEntry {
                            key: PatriciaKeyHex(key),
                            value: FeltHex(value),
                        })
                        .collect(),
                })
                .collect(),
            replaced_classes: vec![],
        };

        Ok(StateUpdate {
            block_hash: FeltHex(state_update.block_hash),
            new_root: FeltHex(state_update.new_root),
            old_root: FeltHex(state_update.old_root),
            state_diff,
        })
    }

    /// starknet_getStorageAt
    pub(crate) async fn get_storage_at(
        &self,
        contract_address: ContractAddressHex,
        key: PatriciaKeyHex,
        block_id: BlockId,
    ) -> RpcResult<FeltHex> {
        let felt = self
            .api
            .starknet
            .read()
            .await
            .contract_storage_at_block(block_id.into(), contract_address.0, key.0)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::StateError(StateError::NoneStorage((_, _)))
                | Error::NoStateAtBlock { block_number: _ } => ApiError::ContractNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(FeltHex(felt))
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
        _block_id: BlockId,
        _class_hash: ClassHashHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ClassHashNotFound)
    }

    /// starknet_getClassHashAt
    pub(crate) async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ClassHashHex> {
        let starknet = self.api.starknet.read().await;
        match starknet.get_class_hash_at(&block_id.into(), &contract_address.0) {
            Ok(class_hash) => Ok(FeltHex(class_hash)),
            Err(
                Error::BlockIdHashUnimplementedError
                | Error::BlockIdNumberUnimplementedError
                | Error::NoBlock,
            ) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound | Error::NoStateAtBlock { block_number: _ }) => {
                Err(ApiError::ContractNotFound)
            }
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getClassAt
    pub(crate) async fn get_class_at(
        &self,
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ContractNotFound)
    }

    /// starknet_getBlockTransactionCount
    pub(crate) async fn get_block_txs_count(&self, _block_id: BlockId) -> RpcResult<BlockNumber> {
        Err(error::ApiError::BlockNotFound)
    }

    /// starknet_call
    pub(crate) async fn call(
        &self,
        block_id: BlockId,
        request: FunctionCall,
    ) -> RpcResult<Vec<FeltHex>> {
        let starknet = self.api.starknet.read().await;
        match starknet.call(
            block_id.into(),
            request.contract_address.0.into(),
            request.entry_point_selector.0,
            request.calldata.iter().map(|c| c.0).collect(),
        ) {
            Ok(result) => Ok(result.into_iter().map(FeltHex).collect()),
            Err(Error::TransactionError(TransactionError::State(
                StateError::NoneContractState(Address(_address)),
            ))) => Err(ApiError::ContractNotFound),
            Err(Error::BlockIdHashUnimplementedError | Error::BlockIdNumberUnimplementedError) => {
                Err(ApiError::OnlyLatestBlock)
            }
            Err(_) => Err(ApiError::ContractError),
        }
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
        let block_number = self.api.starknet.read().await.block_number();
        Ok(block_number)
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
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<FeltHex> {
        let nonce = self
            .api
            .starknet
            .read()
            .await
            .contract_nonce_at_block(block_id.into(), contract_address.0)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoStateAtBlock { block_number: _ } | Error::ContractNotFound => {
                    ApiError::ContractNotFound
                }
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(FeltHex(nonce))
    }
}
