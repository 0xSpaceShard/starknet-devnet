use server::rpc_core::error::RpcError;
use starknet_core::error::Error;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_in_rust::transaction as starknet_in_rust_tx;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_types::felt::Felt;
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::traits::ToHexString;

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
    BroadcastedDeclareTransaction, BroadcastedInvokeTransaction, BroadcastedTransaction,
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
            Err(Error::BlockIdHashUnimplementedError | Error::BlockIdNumberUnimplementedError) => {
                Err(ApiError::BlockNotFound)
            }
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
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
            Err(Error::ContractNotFound) => Err(ApiError::ContractNotFound),
            Err(_) => Err(ApiError::ContractError),
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
            Err(_) => Err(ApiError::ContractError),
            // TODO better handling
        }
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

fn convert_broadcasted_tx(
    broadcasted_tx: BroadcastedTransaction,
    chain_id: StarknetChainId,
) -> RpcResult<starknet_in_rust_tx::Transaction> {
    match broadcasted_tx {
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(_)) => {
            Err(ApiError::UnsupportedAction { msg: "Invoke V0 is not supported".into() })
        }
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(broadcasted_tx)) => {
            let selector =
                Felt::from(starknet_rs_core::utils::get_selector_from_name("__execute__").unwrap())
                    .into();

            Ok(starknet_in_rust_tx::Transaction::InvokeFunction(
                starknet_in_rust_tx::InvokeFunction::new(
                    broadcasted_tx.sender_address.0.try_into()?,
                    selector,
                    broadcasted_tx.common.max_fee.0,
                    broadcasted_tx.common.version.0.into(),
                    broadcasted_tx.calldata.iter().map(|s| s.0.into()).collect(),
                    broadcasted_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                    chain_id.to_felt(),
                    Some(broadcasted_tx.common.nonce.0.into()),
                )?,
            ))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(broadcasted_tx)) => {
            let contract_class: starknet_types::contract_class::ContractClass = broadcasted_tx
                .contract_class
                .try_into() // TODO why does this work if the converter is in write_endpoints.rs?
                .map_err(|_| ApiError::RpcError(RpcError::invalid_request()))?;
            Ok(starknet_in_rust_tx::Transaction::Declare(starknet_in_rust_tx::Declare::new(
                contract_class.try_into()?,
                chain_id.to_felt(),
                broadcasted_tx.sender_address.0.try_into()?,
                broadcasted_tx.common.max_fee.0,
                broadcasted_tx.common.version.0.into(),
                broadcasted_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                broadcasted_tx.common.nonce.0.into(),
            )?))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(broadcasted_tx)) => {
            Ok(starknet_in_rust_tx::Transaction::DeclareV2(Box::new(
                starknet_in_rust_tx::DeclareV2::new(
                    &broadcasted_tx.contract_class,
                    None,
                    broadcasted_tx.compiled_class_hash.0.into(),
                    chain_id.to_felt(),
                    broadcasted_tx.sender_address.0.try_into()?,
                    broadcasted_tx.common.max_fee.0,
                    broadcasted_tx.common.version.0.into(),
                    broadcasted_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                    broadcasted_tx.common.nonce.0.into(),
                )?,
            )))
        }
        BroadcastedTransaction::DeployAccount(broadcasted_tx) => {
            Ok(starknet_in_rust_tx::Transaction::DeployAccount(
                starknet_in_rust_tx::DeployAccount::new(
                    broadcasted_tx.class_hash.0.bytes(),
                    broadcasted_tx.common.max_fee.0,
                    broadcasted_tx.common.version.0.into(),
                    broadcasted_tx.common.nonce.0.into(),
                    broadcasted_tx.constructor_calldata.iter().map(|s| s.0.into()).collect(),
                    broadcasted_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                    broadcasted_tx.contract_address_salt.0.into(),
                    chain_id.to_felt(),
                )?,
            ))
        }
    }
}
