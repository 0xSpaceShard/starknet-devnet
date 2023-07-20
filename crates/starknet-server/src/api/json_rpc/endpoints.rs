use server::rpc_core::error::{ErrorCode, RpcError};
use starknet_core::error::{Error, Result};
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_in_rust::transaction as starknet_in_rust_transaction;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_types::felt::Felt;
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::DevnetResult;

use super::error::{self, ApiError};
use super::models::{BlockHashAndNumberOutput, EstimateFeeOutput, SyncingOutput};
use super::{JsonRpcHandler, RpcResult};
use crate::api::models::block::Block;
use crate::api::models::contract_class::{ContractClass, DeprecatedContractClass};
use crate::api::models::state::ThinStateDiff;
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
        block_id: BlockId,
        request: Vec<BroadcastedTransactionWithType>,
    ) -> RpcResult<Vec<EstimateFeeOutput>> {
        let starknet = self.api.starknet.read().await;
        let transactions = request
            .into_iter()
            .map(|broadcasted_tx| {
                convert_broadcasted_tx(broadcasted_tx.transaction, starknet.config.chain_id)
                    .unwrap() // TODO temporary unwrap - maybe use loop
            })
            .collect();

        match starknet.estimate_fee(block_id.into(), &transactions) {
            Ok(result) => Ok(result
                .iter()
                .map(|(fee, gas_consumed)| EstimateFeeOutput {
                    gas_consumed: format!("0x{gas_consumed:x}"),
                    gas_price: format!("0x{:x}", starknet.config.gas_price),
                    overall_fee: format!("0x{fee:x}"),
                })
                .collect()),
            Err(_) => todo!(),
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

fn convert_broadcasted_tx(
    broadcasted_tx: BroadcastedTransaction,
    chain_id: StarknetChainId,
) -> RpcResult<starknet_in_rust_transaction::Transaction> {
    match broadcasted_tx {
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(_)) => {
            Err(ApiError::UnsupportedAction { msg: "Invoke V0 is not supported".into() })
        }
        BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(invoke_tx)) => {
            let selector =
                Felt::from(starknet_rs_core::utils::get_selector_from_name("__execute__").unwrap())
                    .into();

            Ok(starknet_in_rust_transaction::Transaction::InvokeFunction(
                starknet_in_rust_transaction::InvokeFunction::new(
                    invoke_tx.sender_address.0.try_into()?,
                    selector,
                    invoke_tx.common.max_fee.0,
                    invoke_tx.common.version.0.into(),
                    invoke_tx.calldata.iter().map(|s| s.0.into()).collect(),
                    invoke_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                    chain_id.to_felt(),
                    Some(invoke_tx.common.nonce.0.into()),
                )?,
            ))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(declare_tx)) => {
            let contract_class: starknet_types::contract_class::ContractClass = declare_tx
                .contract_class
                .try_into()
                .map_err(|_| ApiError::RpcError(RpcError::invalid_request()))?;
            Ok(starknet_in_rust_transaction::Transaction::Declare(
                starknet_in_rust_transaction::Declare::new(
                    contract_class.try_into()?,
                    chain_id.to_felt(),
                    declare_tx.sender_address.0.try_into()?,
                    declare_tx.common.max_fee.0,
                    declare_tx.common.version.0.into(),
                    declare_tx.common.signature.iter().map(|s| s.0.into()).collect(),
                    declare_tx.common.nonce.0.into(),
                )?,
            ))
        }
        BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(declare_tx)) => {
            Ok(starknet_in_rust_transaction::Transaction::DeclareV2(Box(
                starknet_in_rust_transaction::DeclareV2::new(
                    sierra_contract_class,
                    casm_contract_class,
                    compiled_class_hash,
                    chain_id,
                    sender_address,
                    max_fee,
                    version,
                    signature,
                    nonce,
                ),
            )))
        }
        BroadcastedTransaction::DeployAccount(_) => todo!(),
    }
}
