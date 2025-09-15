use starknet_core::error::{ContractExecutionError, Error, StateError};
use starknet_rs_core::types::{BlockId as ImportedBlockId, Felt, MsgFromL1};
use starknet_rs_providers::Provider;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{
    Block, BlockHeader, BlockId, BlockResult, BlockStatus, BlockTag, PreConfirmedBlock,
    PreConfirmedBlockHeader,
};
use starknet_types::rpc::state::StateUpdateResult;
use starknet_types::rpc::transactions::{
    BroadcastedTransaction, EventFilter, EventsChunk, FunctionCall, SimulationFlag, Transactions,
};

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    BlockHashAndNumberOutput, GetStorageProofInput, L1TransactionHashInput, SyncingOutput,
};
use super::{DevnetResponse, RPC_SPEC_VERSION, StarknetResponse};
use crate::api::JsonRpcHandler;
use crate::api::endpoints_impl::accounts::{
    BalanceQuery, PredeployedAccountsQuery, get_account_balance_impl, get_predeployed_accounts_impl,
};
use crate::config::DevnetConfig;

const DEFAULT_CONTINUATION_TOKEN: &str = "0";
const CONTINUATION_TOKEN_ORIGIN_PREFIX: &str = "devnet-origin-";

/// The definitions of JSON-RPC read endpoints defined in starknet_api_openrpc.json
impl JsonRpcHandler {
    /// starknet_specVersion
    pub fn spec_version(&self) -> StrictRpcResult {
        Ok(StarknetResponse::String(RPC_SPEC_VERSION.to_string()).into())
    }

    /// starknet_getBlockWithTxHashes
    pub async fn get_block_with_tx_hashes(&self, block_id: BlockId) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;

        let block = starknet.get_block(&block_id).map_err(|err| match err {
            Error::NoBlock => ApiError::BlockNotFound,
            unknown_error => ApiError::StarknetDevnetError(unknown_error),
        })?;

        let transactions = Transactions::Hashes(block.get_transactions().to_owned());

        Ok(match block.status() {
            BlockStatus::PreConfirmed => StarknetResponse::PreConfirmedBlock(PreConfirmedBlock {
                header: PreConfirmedBlockHeader::from(block),
                transactions,
            }),
            _ => StarknetResponse::Block(Block {
                status: *block.status(),
                header: BlockHeader::from(block),
                transactions,
            }),
        }
        .into())
    }

    /// starknet_getBlockWithTxs
    pub async fn get_block_with_txs(&self, block_id: BlockId) -> StrictRpcResult {
        let block = self.api.starknet.lock().await.get_block_with_transactions(&block_id).map_err(
            |err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoTransaction => ApiError::TransactionNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            },
        )?;

        match block {
            BlockResult::Block(b) => Ok(StarknetResponse::Block(b).into()),
            BlockResult::PreConfirmedBlock(b) => Ok(StarknetResponse::PreConfirmedBlock(b).into()),
        }
    }

    /// starknet_getBlockWithReceipts
    pub async fn get_block_with_receipts(&self, block_id: BlockId) -> StrictRpcResult {
        let block = self.api.starknet.lock().await.get_block_with_receipts(&block_id).map_err(
            |e| match e {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoTransaction => ApiError::TransactionNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            },
        )?;

        match block {
            BlockResult::Block(b) => Ok(StarknetResponse::Block(b).into()),
            BlockResult::PreConfirmedBlock(b) => Ok(StarknetResponse::PreConfirmedBlock(b).into()),
        }
    }

    /// starknet_getStateUpdate
    pub async fn get_state_update(&self, block_id: BlockId) -> StrictRpcResult {
        let state_update =
            self.api.starknet.lock().await.block_state_update(&block_id).map_err(|e| match e {
                Error::NoBlock => ApiError::BlockNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        match state_update {
            StateUpdateResult::StateUpdate(s) => Ok(StarknetResponse::StateUpdate(s).into()),
            StateUpdateResult::PreConfirmedStateUpdate(s) => {
                Ok(StarknetResponse::PreConfirmedStateUpdate(s).into())
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
            .contract_storage_at_block(&block_id, contract_address, key)
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

    /// starknet_getStorageProof
    pub async fn get_storage_proof(&self, data: GetStorageProofInput) -> StrictRpcResult {
        match self.api.starknet.lock().await.get_block(&data.block_id) {
            // storage proofs not applicable to Devnet
            Ok(_) => Err(ApiError::StorageProofNotSupported),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
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
            Ok(tx_status) => Ok(StarknetResponse::TransactionStatusByHash(tx_status).into()),
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
        match self.api.starknet.lock().await.get_transaction_by_block_id_and_index(&block_id, index)
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
        match self.api.starknet.lock().await.get_class(&block_id, class_hash) {
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

    /// starknet_getCompiledCasm
    pub async fn get_compiled_casm(&self, class_hash: ClassHash) -> StrictRpcResult {
        // starknet_getCompiledCasm compiles sierra to casm the same way it is done in
        // starknet_addDeclareTransaction, so if during starknet_addDeclareTransaction compilation
        // does not fail, so it will not fail during this endpoint execution
        match self.api.starknet.lock().await.get_compiled_casm(class_hash) {
            Ok(compiled_casm) => Ok(StarknetResponse::CompiledCasm(compiled_casm).into()),
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
        match self.api.starknet.lock().await.get_class_at(&block_id, contract_address) {
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
        match self.api.starknet.lock().await.get_class_hash_at(&block_id, contract_address) {
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
        let num_trans_count = self.api.starknet.lock().await.get_block_txs_count(&block_id);
        match num_trans_count {
            Ok(count) => Ok(StarknetResponse::BlockTransactionCount(count).into()),
            Err(_) => Err(ApiError::BlockNotFound),
        }
    }

    /// starknet_call
    pub async fn call(&self, block_id: BlockId, request: FunctionCall) -> StrictRpcResult {
        match self.api.starknet.lock().await.call(
            &block_id,
            request.contract_address.into(),
            request.entry_point_selector,
            request.calldata,
        ) {
            Ok(result) => Ok(StarknetResponse::Call(result).into()),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::EntrypointNotFound) => Err(ApiError::EntrypointNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(Error::ContractExecutionError(execution_error)) => {
                Err(ApiError::ContractError(execution_error))
            }
            Err(e) => Err(ApiError::ContractError(ContractExecutionError::Message(e.to_string()))),
        }
    }

    /// starknet_estimateFee
    pub async fn estimate_fee(
        &self,
        block_id: BlockId,
        request: Vec<BroadcastedTransaction>,
        simulation_flags: Vec<SimulationFlag>,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.estimate_fee(&block_id, &request, &simulation_flags) {
            Ok(result) => Ok(StarknetResponse::EstimateFee(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(Error::ContractExecutionErrorInSimulation { failure_index, execution_error }) => {
                Err(ApiError::TransactionExecutionError { failure_index, execution_error })
            }
            Err(e) => Err(ApiError::ContractError(ContractExecutionError::from(e.to_string()))),
        }
    }

    pub async fn estimate_message_fee(
        &self,
        block_id: &BlockId,
        message: MsgFromL1,
    ) -> StrictRpcResult {
        match self.api.starknet.lock().await.estimate_message_fee(block_id, message) {
            Ok(result) => Ok(StarknetResponse::EstimateMessageFee(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(Error::ContractExecutionError(error)) => Err(ApiError::ContractError(error)),
            Err(e) => Err(ApiError::ContractError(ContractExecutionError::from(e.to_string()))),
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

    /// Split into origin and local block ranges (non overlapping, local continuing onto origin)
    /// Returns: (origin_range, local_start, local_end)
    /// All ranges inclusive
    async fn split_block_range(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
    ) -> Result<(Option<(u64, u64)>, Option<BlockId>, Option<BlockId>), ApiError> {
        let origin_caller = match &self.origin_caller {
            Some(origin_caller) => origin_caller,
            None => return Ok((None, from_block, to_block)),
        };

        let fork_block_number = origin_caller.fork_block_number();

        let starknet = self.api.starknet.lock().await;

        let from_block_number = match from_block {
            Some(BlockId::Tag(BlockTag::Latest | BlockTag::PreConfirmed)) => {
                return Ok((None, from_block, to_block));
            }
            Some(block_id @ (BlockId::Tag(BlockTag::L1Accepted) | BlockId::Hash(_))) => {
                match starknet.get_block(&block_id) {
                    Ok(block) => block.block_number().0,
                    Err(_) => origin_caller.get_block_number_from_block_id(block_id).await?,
                }
            }
            Some(BlockId::Number(from_block_number)) => from_block_number,
            None => 0, // If no from_block, all blocks before to_block should be queried
        };

        if from_block_number > fork_block_number {
            // Only local blocks need to be searched
            return Ok((None, Some(BlockId::Number(from_block_number)), to_block));
        }

        let to_block_number = match to_block {
            // If to_block is latest, pre_confirmed or undefined, all blocks after from_block are
            // queried
            Some(BlockId::Tag(BlockTag::Latest | BlockTag::PreConfirmed)) | None => {
                return Ok((
                    Some((from_block_number, fork_block_number)),
                    // there is for sure at least one local block
                    Some(BlockId::Number(fork_block_number + 1)),
                    to_block,
                ));
            }
            Some(block_id @ (BlockId::Tag(BlockTag::L1Accepted) | BlockId::Hash(_))) => {
                match starknet.get_block(&block_id) {
                    Ok(block) => block.block_number().0,
                    Err(_) => origin_caller.get_block_number_from_block_id(block_id).await?,
                }
            }
            Some(BlockId::Number(to_block_number)) => to_block_number,
        };

        let origin_range = Some((from_block_number, to_block_number));
        Ok(if to_block_number <= fork_block_number {
            (origin_range, None, None)
        } else {
            (
                origin_range,
                Some(BlockId::Number(fork_block_number + 1)),
                Some(BlockId::Number(to_block_number)),
            )
        })
    }

    /// Fetches events from forking origin. The continuation token should be the same as received by
    /// Devnet (not yet adapted for origin). If more events can be fetched from the origin, this is
    /// noted in the `continuation_token` of the returned `EventsChunk`.
    async fn get_origin_events(
        &self,
        from_origin: u64,
        to_origin: u64,
        continuation_token: Option<String>,
        address: Option<ContractAddress>,
        keys: Option<Vec<Vec<Felt>>>,
        chunk_size: u64,
    ) -> Result<EventsChunk, ApiError> {
        let origin_caller = self.origin_caller.as_ref().ok_or(ApiError::StarknetDevnetError(
            Error::UnexpectedInternalError { msg: "Origin caller unexpectedly undefined".into() },
        ))?;

        let origin_continuation_token = continuation_token
            .map(|token| token.trim_start_matches(CONTINUATION_TOKEN_ORIGIN_PREFIX).to_string());

        let mut origin_events_chunk: EventsChunk = origin_caller
            .starknet_client
            .get_events(
                starknet_rs_core::types::EventFilter {
                    from_block: Some(ImportedBlockId::Number(from_origin)),
                    to_block: Some(ImportedBlockId::Number(to_origin)),
                    address: address.map(|address| address.into()),
                    keys,
                },
                origin_continuation_token,
                chunk_size,
            )
            .await
            .map_err(|e| {
                ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                    msg: format!("Error in fetching origin events: {e:?}"),
                })
            })?
            .into();

        // If origin has no more chunks, set the token to default, which will signalize the
        // switch to querying the local state on next request.
        origin_events_chunk.continuation_token = origin_events_chunk
            .continuation_token
            .map_or(Some(DEFAULT_CONTINUATION_TOKEN.to_owned()), |token| {
                Some(CONTINUATION_TOKEN_ORIGIN_PREFIX.to_owned() + &token)
            });

        Ok(origin_events_chunk)
    }

    /// starknet_getEvents
    pub async fn get_events(&self, filter: EventFilter) -> StrictRpcResult {
        let (origin_range, from_local_block_id, to_local_block_id) =
            self.split_block_range(filter.from_block, filter.to_block).await?;

        // Get events either from forking origin or locally
        let events_chunk = if origin_range.is_some()
            && filter
                .continuation_token
                .clone()
                .is_none_or(|token| token.starts_with(CONTINUATION_TOKEN_ORIGIN_PREFIX))
        {
            #[allow(clippy::expect_used)]
            let (from_origin, to_origin) =
                origin_range.expect("Continuation token implies there are more origin events");

            self.get_origin_events(
                from_origin,
                to_origin,
                filter.continuation_token,
                filter.address,
                filter.keys,
                filter.chunk_size,
            )
            .await?
        } else {
            let pages_read_so_far = filter
                .continuation_token
                .unwrap_or(DEFAULT_CONTINUATION_TOKEN.to_string())
                .parse::<u64>()
                .map_err(|_| ApiError::InvalidContinuationToken)?;

            let starknet = self.api.starknet.lock().await;
            let (events, has_more_events) = starknet
                .get_events(
                    from_local_block_id,
                    to_local_block_id,
                    filter.address,
                    filter.keys,
                    None,
                    pages_read_so_far * filter.chunk_size,
                    Some(filter.chunk_size),
                )
                .map_err(|e| match e {
                    Error::NoBlock => ApiError::BlockNotFound,
                    _ => e.into(),
                })?;

            EventsChunk {
                events,
                continuation_token: has_more_events.then(|| (pages_read_so_far + 1).to_string()),
            }
        };

        Ok(StarknetResponse::Events(events_chunk).into())
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
            .contract_nonce_at_block(&block_id, contract_address)
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
        let mut starknet = self.api.starknet.lock().await;

        match starknet.simulate_transactions(&block_id, &transactions, simulation_flags) {
            Ok(result) => Ok(StarknetResponse::SimulateTransactions(result).into()),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(e @ Error::NoStateAtBlock { .. }) => {
                Err(ApiError::NoStateAtBlock { msg: e.to_string() })
            }
            Err(Error::ContractExecutionErrorInSimulation { failure_index, execution_error }) => {
                Err(ApiError::TransactionExecutionError { failure_index, execution_error })
            }
            Err(e) => Err(ApiError::ContractError(ContractExecutionError::from(e.to_string()))),
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
        match starknet.get_transaction_traces_from_block(&block_id) {
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
    pub async fn get_devnet_config(&self) -> StrictRpcResult {
        Ok(DevnetResponse::DevnetConfig(DevnetConfig {
            starknet_config: self.api.starknet.lock().await.config.clone(),
            server_config: self.server_config.clone(),
        })
        .into())
    }
}
