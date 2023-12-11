use starknet_core::error::{Error, StateError};
use starknet_rs_core::types::MsgFromL1;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{Block, BlockHeader, BlockId};
use starknet_types::rpc::state::StateUpdate;
use starknet_types::rpc::transactions::{
    BroadcastedTransaction, EventFilter, EventsChunk, FunctionCall, SimulationFlag,
};
use starknet_types::traits::ToHexString;

use super::error::{ApiError, StrictRpcResult};
use super::models::{BlockHashAndNumberOutput, SyncingOutput, TransactionStatusOutput};
use super::{JsonRpcHandler, StarknetResponse};

const DEFAULT_CONTINUATION_TOKEN: &str = "0";

/// here are the definitions and stub implementations of all JSON-RPC read endpoints
impl JsonRpcHandler {
    /// starknet_specVersion
    pub(crate) fn spec_version(&self) -> StrictRpcResult {
        Ok(StarknetResponse::SpecVersion(env!("RPC_SPEC_VERSION").to_string()))
    }

    /// starknet_getBlockWithTxHashes
    pub(crate) async fn get_block_with_tx_hashes(&self, block_id: BlockId) -> StrictRpcResult {
        let block =
            self.api.starknet.read().await.get_block(block_id.into()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(StarknetResponse::BlockWithTransactionHashes(Block {
            status: *block.status(),
            header: BlockHeader::from(&block),
            transactions: starknet_types::rpc::transactions::Transactions::Hashes(
                block.get_transactions().to_owned(),
            ),
        }))
    }

    /// starknet_getBlockWithTxs
    pub(crate) async fn get_block_with_txs(&self, block_id: BlockId) -> StrictRpcResult {
        let block =
            self.api.starknet.read().await.get_block_with_transactions(block_id.into()).map_err(
                |err| match err {
                    Error::NoBlock => ApiError::BlockNotFound,
                    Error::NoTransaction => ApiError::TransactionNotFound,
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                },
            )?;

        Ok(StarknetResponse::BlockWithFullTransactions(block))
    }

    /// starknet_getStateUpdate
    pub(crate) async fn get_state_update(&self, block_id: BlockId) -> StrictRpcResult {
        let state_update =
            self.api.starknet.read().await.block_state_update(block_id.into()).map_err(|err| {
                match err {
                    Error::NoBlock => ApiError::BlockNotFound,
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                }
            })?;

        let state_diff = state_update.state_diff.into();

        Ok(StarknetResponse::StateUpdate(StateUpdate {
            block_hash: state_update.block_hash,
            new_root: state_update.new_root,
            old_root: state_update.old_root,
            state_diff,
        }))
    }

    /// starknet_getStorageAt
    pub(crate) async fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: PatriciaKey,
        block_id: BlockId,
    ) -> StrictRpcResult {
        let felt = self
            .api
            .starknet
            .read()
            .await
            .contract_storage_at_block(block_id.into(), contract_address, key)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::StateError(StateError::NoneStorage(_))
                | Error::NoStateAtBlock { block_number: _ } => ApiError::ContractNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(StarknetResponse::StorageAt(felt))
    }

    /// starknet_getTransactionByHash
    pub(crate) async fn get_transaction_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.get_transaction_by_hash(transaction_hash) {
            Ok(transaction) => Ok(StarknetResponse::TransactionByHash(transaction.clone())),
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getTransactionStatus
    pub(crate) async fn get_transaction_status_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self
            .api
            .starknet
            .read()
            .await
            .get_transaction_execution_and_finality_status(transaction_hash)
        {
            Ok((execution_status, finality_status)) => {
                Ok(StarknetResponse::TransactionStatusByHash(TransactionStatusOutput {
                    execution_status,
                    finality_status,
                }))
            }
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getTransactionByBlockIdAndIndex
    pub(crate) async fn get_transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        index: u64,
    ) -> StrictRpcResult {
        match self
            .api
            .starknet
            .read()
            .await
            .get_transaction_by_block_id_and_index(block_id.into(), index)
        {
            Ok(transaction) => {
                Ok(StarknetResponse::TransactionByBlockAndIndex(transaction.clone()))
            }
            Err(Error::InvalidTransactionIndexInBlock) => {
                Err(ApiError::InvalidTransactionIndexInBlock)
            }
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getTransactionReceipt
    pub(crate) async fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.get_transaction_receipt_by_hash(transaction_hash) {
            Ok(receipt) => {
                Ok(StarknetResponse::TransactionReceiptByTransactionHash(Box::new(receipt)))
            }
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getClass
    pub(crate) async fn get_class(
        &self,
        block_id: BlockId,
        class_hash: ClassHash,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.get_class(block_id.into(), class_hash) {
            Ok(contract_class) => Ok(StarknetResponse::ClassByHash(contract_class.try_into()?)),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::StateError(_) | Error::NoStateAtBlock { block_number: _ }) => {
                Err(ApiError::ClassHashNotFound)
            }
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getClassAt
    pub(crate) async fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.get_class_at(block_id.into(), contract_address) {
            Ok(contract_class) => {
                Ok(StarknetResponse::ClassAtContractAddress(contract_class.try_into()?))
            }
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(
                Error::ContractNotFound
                | Error::StateError(_)
                | Error::NoStateAtBlock { block_number: _ },
            ) => Err(ApiError::ContractNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getClassHashAt
    pub(crate) async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.get_class_hash_at(block_id.into(), contract_address) {
            Ok(class_hash) => Ok(StarknetResponse::ClassHashAtContractAddress(class_hash)),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound | Error::NoStateAtBlock { block_number: _ }) => {
                Err(ApiError::ContractNotFound)
            }
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getBlockTransactionCount
    pub(crate) async fn get_block_txs_count(&self, block_id: BlockId) -> StrictRpcResult {
        let num_trans_count = self.api.starknet.read().await.get_block_txs_count(block_id.into());
        match num_trans_count {
            Ok(count) => Ok(StarknetResponse::BlockTransactionCount(count)),
            Err(_) => Err(ApiError::NoBlocks),
        }
    }

    /// starknet_call
    pub(crate) async fn call(&self, block_id: BlockId, request: FunctionCall) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;

        match starknet.call(
            block_id.into(),
            request.contract_address.into(),
            request.entry_point_selector,
            request.calldata,
        ) {
            Ok(result) => Ok(StarknetResponse::Call(result)),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_estimateFee
    pub(crate) async fn estimate_fee(
        &self,
        block_id: BlockId,
        request: Vec<BroadcastedTransaction>,
    ) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;
        match starknet.estimate_fee(block_id.into(), &request) {
            Ok(result) => Ok(StarknetResponse::EsimateFee(result)),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    pub(crate) async fn estimate_message_fee(
        &self,
        block_id: BlockId,
        message: MsgFromL1,
    ) -> StrictRpcResult {
        match self.api.starknet.read().await.estimate_message_fee(block_id.into(), message) {
            Ok(result) => Ok(StarknetResponse::EstimateMessageFee(result)),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_blockNumber
    pub(crate) async fn block_number(&self) -> StrictRpcResult {
        let block = self.api.starknet.read().await.get_latest_block().map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        Ok(StarknetResponse::BlockNumber(block.block_number()))
    }

    /// starknet_blockHashAndNumber
    pub(crate) async fn block_hash_and_number(&self) -> StrictRpcResult {
        let block = self.api.starknet.read().await.get_latest_block().map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        Ok(StarknetResponse::BlockHashAndNumber(BlockHashAndNumberOutput {
            block_hash: block.block_hash(),
            block_number: block.block_number(),
        }))
    }

    /// starknet_chainId
    pub(crate) async fn chain_id(&self) -> StrictRpcResult {
        let chain_id = self.api.starknet.read().await.chain_id();

        Ok(StarknetResponse::ChainId(chain_id.to_felt().to_prefixed_hex_str()))
    }

    /// starknet_syncing
    pub(crate) async fn syncing(&self) -> StrictRpcResult {
        Ok(StarknetResponse::Syncing(SyncingOutput::False(false)))
    }

    /// starknet_getEvents
    pub(crate) async fn get_events(&self, filter: EventFilter) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;

        let page = filter
            .continuation_token
            .unwrap_or(DEFAULT_CONTINUATION_TOKEN.to_string())
            .parse::<usize>()
            .map_err(|_| ApiError::InvalidContinuationToken)?;

        let (events, has_more_events) = starknet.get_events(
            filter.from_block,
            filter.to_block,
            filter.address,
            filter.keys,
            page * filter.chunk_size,
            Some(filter.chunk_size),
        )?;

        Ok(StarknetResponse::Events(EventsChunk {
            events,
            continuation_token: if has_more_events { Some((page + 1).to_string()) } else { None },
        }))
    }

    /// starknet_getNonce
    pub(crate) async fn get_nonce(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        let nonce = self
            .api
            .starknet
            .read()
            .await
            .contract_nonce_at_block(block_id.into(), contract_address)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoStateAtBlock { block_number: _ } | Error::ContractNotFound => {
                    ApiError::ContractNotFound
                }
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(StarknetResponse::ContractNonce(nonce))
    }

    /// starknet_simulateTransactions
    pub(crate) async fn simulate_transactions(
        &self,
        block_id: BlockId,
        transactions: Vec<BroadcastedTransaction>,
        simulation_flags: Vec<SimulationFlag>,
    ) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;
        match starknet.simulate_transactions(block_id.into(), &transactions, simulation_flags) {
            Ok(result) => Ok(StarknetResponse::SimulateTransactions(result)),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_traceTransaction
    pub(crate) async fn get_trace_transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;
        match starknet.get_transaction_trace_by_hash(transaction_hash) {
            Ok(result) => Ok(StarknetResponse::TraceTransaction(result)),
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(Error::UnsupportedTransactionType) => Err(ApiError::NoTraceAvailable),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_traceBlockTransactions
    pub(crate) async fn get_trace_block_transactions(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.read().await;
        match starknet.get_transaction_traces_from_block(block_id.into()) {
            Ok(result) => Ok(StarknetResponse::BlockTransactionTraces(result)),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(err) => Err(err.into()),
        }
    }
}
