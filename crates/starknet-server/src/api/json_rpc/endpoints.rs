use starknet_core::error::Error;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::starknet_api::transaction::Fee;
use starknet_types::traits::ToHexString;

use super::error::{self, ApiError};
use super::models::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};
use super::{JsonRpcHandler, RpcResult};
use crate::api::models::block::{Block, BlockHeader};
use crate::api::models::contract_class::ContractClass;
use crate::api::models::state::{
    ClassHashes, ContractNonce, DeployedContract, StateUpdate, StorageDiff, StorageEntry,
    ThinStateDiff,
};
use crate::api::models::transaction::{
    BroadcastedTransactionWithType, DeclareTransactionV0V1, DeclareTransactionV2, EmittedEvent,
    Event, EventContent, EventFilter, EventsChunk, FunctionCall, Transaction, TransactionReceipt,
    TransactionType, TransactionWithType, Transactions,
};
use crate::api::models::{BlockId, ContractAddressHex, PatriciaKeyHex};

/// here are the definitions and stub implementations of all JSON-RPC read endpoints
impl JsonRpcHandler {
    /// starknet_getBlockWithTxHashes
    pub(crate) async fn get_block_with_tx_hashes(&self, block_id: BlockId) -> RpcResult<Block> {
        let block =
            self.api.starknet.read().await.get_block(block_id.into()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(Block {
            status: *block.status(),
            header: BlockHeader::from(&block),
            transactions: crate::api::models::transaction::Transactions::Hashes(
                block.get_transactions().to_owned(),
            ),
        })
    }

    /// starknet_getBlockWithTxs
    pub(crate) async fn get_block_with_txs(&self, block_id: BlockId) -> RpcResult<Block> {
        let starknet = self.api.starknet.read().await;
        let block = starknet.get_block(block_id.into()).map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        let mut transactions = Vec::<TransactionWithType>::new();

        for txn_hash in block.get_transactions() {
            let txn_to_add =
                TransactionWithType::try_from(starknet.get_transaction_by_hash(*txn_hash)?)?;

            transactions.push(txn_to_add);
        }
        Ok(Block {
            status: *block.status(),
            header: BlockHeader::from(&block),
            transactions: Transactions::Full(transactions),
        })
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
                    class_hash,
                })
                .collect(),
            declared_classes: state_update
                .declared_classes
                .into_iter()
                .map(|(class_hash, compiled_class_hash)| ClassHashes {
                    class_hash,
                    compiled_class_hash,
                })
                .collect(),
            deprecated_declared_classes: state_update.cairo_0_declared_classes,
            nonces: state_update
                .nonces
                .into_iter()
                .map(|(address, nonce)| ContractNonce {
                    contract_address: ContractAddressHex(address),
                    nonce,
                })
                .collect(),
            storage_diffs: state_update
                .storage_updates
                .into_iter()
                .map(|(contract_address, updates)| StorageDiff {
                    address: ContractAddressHex(contract_address),
                    storage_entries: updates
                        .into_iter()
                        .map(|(key, value)| StorageEntry { key: PatriciaKeyHex(key), value })
                        .collect(),
                })
                .collect(),
            replaced_classes: vec![],
        };

        Ok(StateUpdate {
            block_hash: state_update.block_hash,
            new_root: state_update.new_root,
            old_root: state_update.old_root,
            state_diff,
        })
    }

    /// starknet_getStorageAt
    pub(crate) async fn get_storage_at(
        &self,
        contract_address: ContractAddressHex,
        key: PatriciaKeyHex,
        block_id: BlockId,
    ) -> RpcResult<Felt> {
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

        Ok(felt)
    }

    /// starknet_getTransactionByHash
    pub(crate) async fn get_transaction_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> RpcResult<TransactionWithType> {
        let starknet = self.api.starknet.read().await;
        let transaction_to_map = starknet
            .transactions
            .get(&transaction_hash)
            .ok_or(error::ApiError::TransactionNotFound)?;
        let transaction_type;
        let transaction_data: Transaction = match transaction_to_map.inner.clone() {
            starknet_core::transactions::Transaction::Declare(declare_v1) => {
                transaction_type = TransactionType::Declare;
                Transaction::Declare(crate::api::models::transaction::DeclareTransaction::Version1(
                    DeclareTransactionV0V1 {
                        class_hash: declare_v1.class_hash,
                        sender_address: ContractAddressHex(declare_v1.sender_address),
                        nonce: declare_v1.nonce,
                        max_fee: Fee(declare_v1.max_fee),
                        version: Felt::from(1),
                        transaction_hash: declare_v1.transaction_hash,
                        signature: declare_v1.signature,
                    },
                ))
            }
            starknet_core::transactions::Transaction::DeclareV2(declare_v2) => {
                transaction_type = TransactionType::Declare;
                Transaction::Declare(crate::api::models::transaction::DeclareTransaction::Version2(
                    DeclareTransactionV2 {
                        class_hash: declare_v2.class_hash,
                        sender_address: ContractAddressHex(declare_v2.sender_address),
                        nonce: declare_v2.nonce,
                        max_fee: Fee(declare_v2.max_fee),
                        version: Felt::from(2),
                        transaction_hash: declare_v2.transaction_hash,
                        signature: declare_v2.signature,
                        compiled_class_hash: declare_v2.compiled_class_hash,
                    },
                ))
            }
            starknet_core::transactions::Transaction::DeployAccount(_deploy) => {
                return Err(error::ApiError::TransactionNotFound);
            }
            starknet_core::transactions::Transaction::Invoke(_invoke) => {
                return Err(error::ApiError::TransactionNotFound);
            }
        };

        let transaction =
            TransactionWithType { transaction: transaction_data, r#type: transaction_type };

        Ok(transaction)
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
        _transaction_hash: TransactionHash,
    ) -> RpcResult<TransactionReceipt> {
        Err(error::ApiError::TransactionNotFound)
    }

    /// starknet_getClass
    pub(crate) async fn get_class(
        &self,
        _block_id: BlockId,
        _class_hash: ClassHash,
    ) -> RpcResult<ContractClass> {
        Err(error::ApiError::ClassHashNotFound)
    }

    /// starknet_getClassHashAt
    pub(crate) async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<ClassHash> {
        let starknet = self.api.starknet.read().await;
        match starknet.get_class_hash_at(&block_id.into(), &contract_address.0) {
            Ok(class_hash) => Ok(class_hash),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
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
    pub(crate) async fn get_block_txs_count(&self, block_id: BlockId) -> RpcResult<u64> {
        let num_trans_count = self.api.starknet.read().await.get_block_txs_count(block_id.into());
        match num_trans_count {
            Ok(count) => Ok(count),
            Err(_) => Err(ApiError::NoBlocks),
        }
    }

    /// starknet_call
    pub(crate) async fn call(
        &self,
        block_id: BlockId,
        request: FunctionCall,
    ) -> RpcResult<Vec<Felt>> {
        let starknet = self.api.starknet.read().await;
        match starknet.call(
            block_id.into(),
            request.contract_address.0.into(),
            request.entry_point_selector,
            request.calldata,
        ) {
            Ok(result) => Ok(result),
            Err(Error::TransactionError(TransactionError::State(
                StateError::NoneContractState(Address(_address)),
            ))) => Err(ApiError::ContractNotFound),
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
        let block = self.api.starknet.read().await.get_latest_block().map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        Ok(BlockHashAndNumberOutput {
            block_hash: block.block_hash(),
            block_number: block.block_number(),
        })
    }

    /// starknet_chainId
    pub(crate) async fn chain_id(&self) -> RpcResult<String> {
        let chain_id = self.api.starknet.read().await.chain_id();

        Ok(Felt::from(chain_id.to_felt()).to_prefixed_hex_str())
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
    pub(crate) async fn get_events(&self, filter: EventFilter) -> RpcResult<EventsChunk> {
        let starknet = self.api.starknet.read().await;

        let skip = filter
            .continuation_token
            .unwrap_or("0".to_string())
            .parse::<usize>()
            .map_err(|_| ApiError::InvalidContinuationToken)?;

        let (events, has_more_events) = starknet.get_events(
            filter.from_block,
            filter.to_block,
            filter.address.map(|val| val.0),
            filter.keys,
            skip,
            Some(filter.chunk_size),
        )?;

        Ok(EventsChunk {
            events: events
                .into_iter()
                .map(|emitted_event| EmittedEvent {
                    block_hash: emitted_event.block_hash,
                    block_number: emitted_event.block_number,
                    transaction_hash: emitted_event.transaction_hash,
                    event: Event {
                        from_address: ContractAddressHex(emitted_event.from_address),
                        content: EventContent {
                            keys: emitted_event.keys,
                            data: emitted_event.data,
                        },
                    },
                })
                .collect(),
            continuation_token: if has_more_events { Some((skip + 1).to_string()) } else { None },
        })
    }

    /// starknet_getNonce
    pub(crate) async fn get_nonce(
        &self,
        block_id: BlockId,
        contract_address: ContractAddressHex,
    ) -> RpcResult<Felt> {
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

        Ok(nonce)
    }
}
