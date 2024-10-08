use starknet_core::error::{Error, StateError};
use starknet_rs_core::types::{BlockId as ImportedBlockId, MsgFromL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{
    Block, BlockHeader, BlockId, BlockResult, PendingBlock, PendingBlockHeader,
};
use starknet_types::rpc::state::StateUpdateResult;
use starknet_types::rpc::transactions::{
    BroadcastedTransaction, EventFilter, EventsChunk, FunctionCall, SimulationFlag,
};
use starknet_types::starknet_api::block::BlockStatus;

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    BlockHashAndNumberOutput, L1TransactionHashInput, SyncingOutput, TransactionStatusOutput,
};
use super::{DevnetResponse, JsonRpcHandler, JsonRpcResponse, StarknetResponse, RPC_SPEC_VERSION};
use crate::api::http::endpoints::accounts::{
    get_account_balance_impl, get_predeployed_accounts_impl, BalanceQuery, PredeployedAccountsQuery,
};
use crate::api::http::endpoints::DevnetConfig;

const DEFAULT_CONTINUATION_TOKEN: &str = "0";

/// here are the definitions and stub implementations of all JSON-RPC read endpoints
impl JsonRpcHandler {
    /// starknet_specVersion
    pub fn spec_version(&self) -> StrictRpcResult {
        Ok(StarknetResponse::String(RPC_SPEC_VERSION.to_string()).into())
    }

    /// starknet_getBlockWithTxHashes
    pub async fn get_block_with_tx_hashes(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;

        let block = starknet.get_block(block_id.as_ref()).map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        if block.status() == &BlockStatus::Pending {
            Ok(StarknetResponse::PendingBlock(PendingBlock {
                header: PendingBlockHeader::from(block),
                transactions: starknet_types::rpc::transactions::Transactions::Hashes(
                    block.get_transactions().to_owned(),
                ),
            })
            .into())
        } else {
            Ok(StarknetResponse::Block(Block {
                status: *block.status(),
                header: BlockHeader::from(block),
                transactions: starknet_types::rpc::transactions::Transactions::Hashes(
                    block.get_transactions().to_owned(),
                ),
            })
            .into())
        }
    }

    /// starknet_getBlockWithTxs
    pub async fn get_block_with_txs(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;

        let block =
            starknet.get_block_with_transactions(block_id.as_ref()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoTransaction => ApiError::TransactionNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        match block {
            BlockResult::Block(b) => Ok(StarknetResponse::Block(b).into()),
            BlockResult::PendingBlock(b) => Ok(StarknetResponse::PendingBlock(b).into()),
        }
    }

    /// starknet_getBlockWithReceipts
    pub async fn get_block_with_receipts(&self, block_id: BlockId) -> StrictRpcResult {
        let block =
            self.api.starknet.lock().await.get_block_with_receipts(block_id.as_ref()).map_err(
                |err| match err {
                    Error::NoBlock => ApiError::BlockNotFound,
                    Error::NoTransaction => ApiError::TransactionNotFound,
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                },
            )?;

        match block {
            BlockResult::Block(b) => Ok(StarknetResponse::Block(b).into()),
            BlockResult::PendingBlock(b) => Ok(StarknetResponse::PendingBlock(b).into()),
        }
    }

    /// starknet_getStateUpdate
    pub async fn get_state_update(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;

        let state_update =
            starknet.block_state_update(block_id.as_ref()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        match state_update {
            StateUpdateResult::StateUpdate(s) => Ok(StarknetResponse::StateUpdate(s).into()),
            StateUpdateResult::PendingStateUpdate(s) => {
                Ok(StarknetResponse::PendingStateUpdate(s).into())
            }
        }
    }

    /// starknet_getStorageAt
    pub async fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: PatriciaKey,
        block_id: BlockId,
    ) -> StrictRpcResult {
        let felt = self
            .api
            .starknet
            .lock()
            .await
            .contract_storage_at_block(block_id.as_ref(), contract_address, key)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::ContractNotFound | Error::StateError(StateError::NoneStorage(_)) => {
                    ApiError::ContractNotFound
                }
                e @ Error::NoStateAtBlock { .. } => ApiError::NoStateAtBlock { msg: e.to_string() },
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(StarknetResponse::Felt(felt).into())
    }

    /// starknet_getTransactionByHash
    pub async fn get_transaction_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_transaction_by_hash(transaction_hash) {
            Ok(transaction) => Ok(StarknetResponse::Transaction(transaction.clone()).into()),
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getTransactionStatus
    pub async fn get_transaction_status_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self
            .api
            .starknet
            .lock()
            .await
            .get_transaction_execution_and_finality_status(transaction_hash)
        {
            Ok((execution_status, finality_status)) => {
                Ok(StarknetResponse::TransactionStatusByHash(TransactionStatusOutput {
                    execution_status,
                    finality_status,
                })
                .into())
            }
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getTransactionByBlockIdAndIndex
    pub async fn get_transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        index: u64,
    ) -> StrictRpcResult {
        match self
            .api
            .starknet
            .lock()
            .await
            .get_transaction_by_block_id_and_index(block_id.as_ref(), index)
        {
            Ok(transaction) => Ok(StarknetResponse::Transaction(transaction.clone()).into()),
            Err(Error::InvalidTransactionIndexInBlock) => {
                Err(ApiError::InvalidTransactionIndexInBlock)
            }
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getTransactionReceipt
    pub async fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_transaction_receipt_by_hash(&transaction_hash) {
            Ok(receipt) => {
                Ok(StarknetResponse::TransactionReceiptByTransactionHash(Box::new(receipt)).into())
            }
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getClass
    pub async fn get_class(&self, block_id: BlockId, class_hash: ClassHash) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_class(block_id.as_ref(), class_hash) {
            Ok(contract_class) => {
                Ok(StarknetResponse::ContractClass(contract_class.try_into()?).into())
            }
            Err(e) => Err(match e {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::StateError(_) => ApiError::ClassHashNotFound,
                e @ Error::NoStateAtBlock { .. } => ApiError::NoStateAtBlock { msg: e.to_string() },
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            }),
        }
    }

    /// starknet_getClassAt
    pub async fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_class_at(block_id.as_ref(), contract_address) {
            Ok(contract_class) => {
                Ok(StarknetResponse::ContractClass(contract_class.try_into()?).into())
            }
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::StateError(StateError::NoneClassHash(_))) => {
                // NoneClassHash can be returned only when forking, otherwise it means that
                // contract_address is locally present, but its class hash isn't, which is a bug.
                // ClassHashNotFound is not expected to be returned by the server, but to be handled
                // by the forking logic as a signal to forward the request to the origin.
                Err(ApiError::ClassHashNotFound)
            }
            Err(Error::ContractNotFound | Error::StateError(_)) => Err(ApiError::ContractNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getClassHashAt
    pub async fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_class_hash_at(block_id.as_ref(), contract_address)
        {
            Ok(class_hash) => Ok(StarknetResponse::Felt(class_hash).into()),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getBlockTransactionCount
    pub async fn get_block_txs_count(&self, block_id: BlockId) -> StrictRpcResult {
        let num_trans_count = self.api.starknet.lock().await.get_block_txs_count(block_id.as_ref());
        match num_trans_count {
            Ok(count) => Ok(StarknetResponse::BlockTransactionCount(count).into()),
            Err(_) => Err(ApiError::BlockNotFound),
        }
    }

    /// starknet_call
    pub async fn call(&self, block_id: BlockId, request: FunctionCall) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;

        match starknet.call(
            block_id.as_ref(),
            request.contract_address.into(),
            request.entry_point_selector,
            request.calldata,
        ) {
            Ok(result) => Ok(StarknetResponse::Call(result).into()),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_estimateFee
    pub async fn estimate_fee(
        &self,
        block_id: BlockId,
        request: Vec<BroadcastedTransaction>,
        simulation_flags: Vec<SimulationFlag>,
    ) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        match starknet.estimate_fee(block_id.as_ref(), &request, &simulation_flags) {
            Ok(result) => Ok(StarknetResponse::EstimateFee(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    pub async fn estimate_message_fee(
        &self,
        block_id: &ImportedBlockId,
        message: MsgFromL1,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.estimate_message_fee(block_id, message) {
            Ok(result) => Ok(StarknetResponse::EstimateMessageFee(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_blockNumber
    pub async fn block_number(&self) -> StrictRpcResult {
        let block = self.api.starknet.lock().await.get_latest_block().map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        Ok(StarknetResponse::BlockNumber(block.block_number()).into())
    }

    /// starknet_blockHashAndNumber
    pub async fn block_hash_and_number(&self) -> StrictRpcResult {
        let block = self.api.starknet.lock().await.get_latest_block().map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        Ok(StarknetResponse::BlockHashAndNumber(BlockHashAndNumberOutput {
            block_hash: block.block_hash(),
            block_number: block.block_number(),
        })
        .into())
    }

    /// starknet_chainId
    pub async fn chain_id(&self) -> StrictRpcResult {
        let chain_id = self.api.starknet.lock().await.chain_id();

        Ok(StarknetResponse::Felt(chain_id.to_felt()).into())
    }

    /// starknet_syncing
    pub async fn syncing(&self) -> StrictRpcResult {
        Ok(StarknetResponse::Syncing(SyncingOutput::False(false)).into())
    }

    /// starknet_getEvents
    pub async fn get_events(&self, filter: EventFilter) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;

        let page = filter
            .continuation_token
            .unwrap_or(DEFAULT_CONTINUATION_TOKEN.to_string())
            .parse::<usize>()
            .map_err(|_| ApiError::InvalidContinuationToken)?;

        let (events, has_more_events) = starknet
            .get_events(
                filter.from_block,
                filter.to_block,
                filter.address,
                filter.keys,
                page * filter.chunk_size,
                Some(filter.chunk_size),
            )
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                _ => err.into(),
            })?;

        Ok(StarknetResponse::Events(EventsChunk {
            events,
            continuation_token: if has_more_events { Some((page + 1).to_string()) } else { None },
        })
        .into())
    }

    /// starknet_getNonce
    pub async fn get_nonce(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> StrictRpcResult {
        let nonce = self
            .api
            .starknet
            .lock()
            .await
            .contract_nonce_at_block(block_id.as_ref(), contract_address)
            .map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::ContractNotFound => ApiError::ContractNotFound,
                e @ Error::NoStateAtBlock { .. } => ApiError::NoStateAtBlock { msg: e.to_string() },
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(StarknetResponse::Felt(nonce).into())
    }

    /// starknet_simulateTransactions
    pub async fn simulate_transactions(
        &self,
        block_id: BlockId,
        transactions: Vec<BroadcastedTransaction>,
        simulation_flags: Vec<SimulationFlag>,
    ) -> StrictRpcResult {
        // borrowing as write/mutable because trace calculation requires so
        let mut starknet = self.api.starknet.lock().await;
        match starknet.simulate_transactions(block_id.as_ref(), &transactions, simulation_flags) {
            Ok(result) => Ok(StarknetResponse::SimulateTransactions(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(err) => Err(ApiError::ContractError { error: err }),
        }
    }

    /// starknet_traceTransaction
    pub async fn get_trace_transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;
        match starknet.get_transaction_trace_by_hash(transaction_hash) {
            Ok(result) => Ok(StarknetResponse::TraceTransaction(result).into()),
            Err(Error::NoTransaction) => Err(ApiError::TransactionNotFound),
            Err(Error::UnsupportedTransactionType) => Err(ApiError::NoTraceAvailable),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_traceBlockTransactions
    pub async fn get_trace_block_transactions(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;
        match starknet.get_transaction_traces_from_block(block_id.as_ref()) {
            Ok(result) => Ok(StarknetResponse::BlockTransactionTraces(result).into()),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(err) => Err(err.into()),
        }
    }

    /// starknet_getMessagesStatus
    pub async fn get_messages_status(
        &self,
        L1TransactionHashInput { transaction_hash }: L1TransactionHashInput,
    ) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;
        match starknet.get_messages_status(transaction_hash) {
            Some(statuses) => Ok(StarknetResponse::MessagesStatusByL1Hash(statuses).into()),
            None => Err(ApiError::TransactionNotFound),
        }
    }

    /// devnet_getPredeployedAccounts
    pub async fn get_predeployed_accounts(
        &self,
        params: Option<PredeployedAccountsQuery>,
    ) -> StrictRpcResult {
        let predeployed_accounts = get_predeployed_accounts_impl(
            &self.api,
            params.unwrap_or(PredeployedAccountsQuery { with_balance: Option::None }),
        )
        .await
        .map_err(ApiError::from)?;

        Ok(DevnetResponse::PredeployedAccounts(predeployed_accounts).into())
    }

    /// devnet_getAccountBalance
    pub async fn get_account_balance(&self, params: BalanceQuery) -> StrictRpcResult {
        let account_balance =
            get_account_balance_impl(&self.api, params).await.map_err(ApiError::from)?;

        Ok(DevnetResponse::AccountBalance(account_balance).into())
    }

    /// devnet_getConfig
    pub async fn get_devnet_config(&self) -> Result<JsonRpcResponse, ApiError> {
        Ok(DevnetResponse::DevnetConfig(DevnetConfig {
            starknet_config: self.api.starknet.lock().await.config.clone(),
            server_config: self.server_config.clone(),
        })
        .into())
    }
}
