use starknet_core::error::Error;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_rs_core::types::ContractClass as CodegenContractClass;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::traits::ToHexString;

use super::error::{self, ApiError};
use super::models::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};
use super::write_endpoints::{
    convert_to_declare_transaction_v1, convert_to_declare_transaction_v2,
    convert_to_deploy_account_transaction, convert_to_invoke_transaction_v1,
};
use super::{JsonRpcHandler, RpcResult};
use crate::api::models::block::{Block, BlockHeader};
use crate::api::models::state::{
    ClassHashes, ContractNonce, DeployedContract, StateUpdate, StorageDiff, StorageEntry,
    ThinStateDiff,
};
use crate::api::models::transaction::{
    BroadcastedDeclareTransaction, BroadcastedInvokeTransaction, BroadcastedTransaction,
    BroadcastedTransactionWithType, EventFilter, EventsChunk, FunctionCall, Transaction,
    TransactionReceiptWithStatus, TransactionWithType, Transactions,
};
use crate::api::models::{BlockId, PatriciaKeyHex};

const DEFAULT_CONTINUATION_TOKEN: &str = "0";

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
        let (block, transactions) =
            starknet.get_block_with_transactions(block_id.into()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                Error::NoTransaction => ApiError::TransactionNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        let mut transactions_with_type = Vec::<TransactionWithType>::new();

        for transaction in transactions {
            let txn_to_add = TransactionWithType::try_from(transaction)?;

            transactions_with_type.push(txn_to_add);
        }
        Ok(Block {
            status: *block.status(),
            header: BlockHeader::from(&block),
            transactions: Transactions::Full(transactions_with_type),
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
                .map(|(address, class_hash)| DeployedContract { address, class_hash })
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
                .map(|(address, nonce)| ContractNonce { contract_address: address, nonce })
                .collect(),
            storage_diffs: state_update
                .storage_updates
                .into_iter()
                .map(|(contract_address, updates)| StorageDiff {
                    address: contract_address,
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
        contract_address: ContractAddress,
        key: PatriciaKeyHex,
        block_id: BlockId,
    ) -> RpcResult<Felt> {
        let felt = self
            .api
            .starknet
            .read()
            .await
            .contract_storage_at_block(block_id.into(), contract_address, key.0)
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
        let transaction = TransactionWithType::try_from(&transaction_to_map.inner)?;

        Ok(transaction)
    }

    /// starknet_getTransactionByBlockIdAndIndex
    pub(crate) async fn get_transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        index: u64,
    ) -> RpcResult<TransactionWithType> {
        let starknet = self.api.starknet.read().await;
        match starknet.get_block(block_id.into()) {
            Ok(block) => {
                let transaction_hash = block
                    .get_transactions()
                    .get(index as usize)
                    .ok_or(error::ApiError::InvalidTransactionIndexInBlock)?;

                let transaction = starknet.get_transaction_by_hash(*transaction_hash)?;
                TransactionWithType::try_from(transaction)
            }
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getTransactionReceipt
    pub(crate) async fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> RpcResult<TransactionReceiptWithStatus> {
        let starknet = self.api.starknet.read().await;
        let transaction_to_map = starknet
            .transactions
            .get(&transaction_hash)
            .ok_or(error::ApiError::TransactionNotFound)?;
        let receipt_with_status = TransactionReceiptWithStatus::try_from(transaction_to_map)?;

        Ok(receipt_with_status)
    }

    /// starknet_getClass
    pub(crate) async fn get_class(
        &self,
        block_id: BlockId,
        class_hash: ClassHash,
    ) -> RpcResult<CodegenContractClass> {
        match self.api.starknet.read().await.get_class(block_id.into(), class_hash) {
            Ok(contract_class) => Ok(contract_class.try_into()?),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(
                Error::ContractNotFound
                | Error::StateError(StateError::NoneContractState(_))
                | Error::NoStateAtBlock { block_number: _ },
            ) => Err(ApiError::ContractNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
    }

    /// starknet_getClassAt
    pub(crate) async fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> RpcResult<CodegenContractClass> {
        match self.api.starknet.read().await.get_class_at(block_id.into(), contract_address) {
            Ok(contract_class) => Ok(contract_class.try_into()?),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(
                Error::ContractNotFound
                | Error::StateError(StateError::NoneContractState(_))
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
    ) -> RpcResult<ClassHash> {
        match self.api.starknet.read().await.get_class_hash_at(block_id.into(), contract_address) {
            Ok(class_hash) => Ok(class_hash),
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(
                Error::ContractNotFound
                | Error::StateError(StateError::NoneContractState(_))
                | Error::NoStateAtBlock { block_number: _ },
            ) => Err(ApiError::ContractNotFound),
            Err(unknown_error) => Err(ApiError::StarknetDevnetError(unknown_error)),
        }
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
            request.contract_address.into(),
            request.entry_point_selector,
            request.calldata,
        ) {
            Ok(result) => Ok(result),
            Err(Error::TransactionError(TransactionError::State(
                StateError::NoneContractState(Address(_address)),
            ))) => Err(ApiError::ContractNotFound),
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(err) => Err(ApiError::ContractError { msg: err.to_string() }),
        }
    }

    /// starknet_estimateFee
    pub(crate) async fn estimate_fee(
        &self,
        block_id: BlockId,
        request: Vec<BroadcastedTransactionWithType>,
    ) -> RpcResult<Vec<EstimateFeeOutput>> {
        let starknet = self.api.starknet.read().await;
        let mut transactions = vec![];
        for broadcasted_tx in request {
            transactions.push(convert_broadcasted_tx(
                broadcasted_tx.transaction,
                starknet.config.chain_id,
            )?);
        }

        match starknet.estimate_gas_usage(block_id.into(), &transactions) {
            Ok(result) => Ok(result
                .iter()
                .map(|gas_consumed| EstimateFeeOutput {
                    gas_consumed: format!("0x{gas_consumed:x}"),
                    gas_price: format!("0x{:x}", starknet.config.gas_price),
                    overall_fee: format!("0x{:x}", starknet.config.gas_price * gas_consumed),
                })
                .collect()),
            Err(err) => Err(ApiError::ContractError { msg: err.to_string() }),
        }
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

        Ok(EventsChunk {
            events,
            continuation_token: if has_more_events { Some((page + 1).to_string()) } else { None },
        })
    }

    /// starknet_getNonce
    pub(crate) async fn get_nonce(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> RpcResult<Felt> {
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

        Ok(nonce)
    }
}

fn convert_broadcasted_tx(
    broadcasted_tx: BroadcastedTransaction,
    chain_id: StarknetChainId,
) -> RpcResult<starknet_core::transactions::Transaction> {
    let chain_id = Felt::from(chain_id.to_felt());
    match broadcasted_tx {
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(_)) => {
            Err(ApiError::UnsupportedAction { msg: "Invoke V0 is not supported".into() })
        }
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(broadcasted_tx)) => {
            Ok(starknet_core::transactions::Transaction::Invoke(Box::new(
                convert_to_invoke_transaction_v1(broadcasted_tx, chain_id)?,
            )))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(broadcasted_tx)) => {
            Ok(starknet_core::transactions::Transaction::Declare(Box::new(
                convert_to_declare_transaction_v1(*broadcasted_tx, chain_id)?,
            )))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(broadcasted_tx)) => {
            Ok(starknet_core::transactions::Transaction::DeclareV2(Box::new(
                convert_to_declare_transaction_v2(*broadcasted_tx, chain_id)?,
            )))
        }
        BroadcastedTransaction::DeployAccount(broadcasted_tx) => {
            Ok(starknet_core::transactions::Transaction::DeployAccount(Box::new(
                convert_to_deploy_account_transaction(broadcasted_tx, chain_id)?,
            )))
        }
    }
}
