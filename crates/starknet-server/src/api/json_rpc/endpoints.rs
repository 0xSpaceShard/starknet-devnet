use starknet_core::error::Error;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::utils::Address;
use starknet_types::felt::Felt;
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::starknet_api::transaction::Fee;

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
    BroadcastedTransactionWithType, ClassHashHex, DeclareTransaction, DeclareTransactionV0V1,
    DeclareTransactionV2, DeployAccountTransaction, EventFilter, EventsChunk, FunctionCall,
    InvokeTransactionV1, Transaction, TransactionHashHex, TransactionReceipt, TransactionType,
    TransactionWithType, Transactions,
};
use crate::api::models::{BlockId, ContractAddressHex, FeltHex, PatriciaKeyHex};
use crate::api::utils::into_vec;

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
                block
                    .get_transactions()
                    .iter()
                    // We shouldnt get in the situation where tx hash is None
                    .map(|tx| FeltHex(tx.get_hash().unwrap_or_default()))
                    .collect(),
            ),
        })
    }

    /// starknet_getBlockWithTxs
    pub(crate) async fn get_block_with_txs(&self, block_id: BlockId) -> RpcResult<Block> {
        let block =
            self.api.starknet.read().await.get_block(block_id.into()).map_err(|err| match err {
                Error::NoBlock => ApiError::BlockNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        let mut transactions = Vec::<TransactionWithType>::new();

        for txn in block.get_transactions() {
            let txn_to_add = match txn {
                starknet_core::transactions::Transaction::Declare(declare_v1) => {
                    let declare_txn = DeclareTransactionV0V1 {
                        class_hash: declare_v1.class_hash().unwrap_or(&Felt::default()).into(),
                        sender_address: declare_v1.sender_address().into(),
                        nonce: txn.nonce().into(),
                        max_fee: Fee(txn.max_fee()),
                        version: txn.version().into(),
                        transaction_hash: txn.get_hash().unwrap_or_default().into(),
                        signature: into_vec(txn.signature()),
                    };
                    TransactionWithType {
                        r#type: TransactionType::Declare,
                        transaction: Transaction::Declare(DeclareTransaction::Version1(
                            declare_txn,
                        )),
                    }
                }
                starknet_core::transactions::Transaction::DeclareV2(declare_v2) => {
                    let declare_txn = DeclareTransactionV2 {
                        class_hash: declare_v2.class_hash().unwrap_or(&Felt::default()).into(),
                        compiled_class_hash: declare_v2.compiled_class_hash().into(),
                        sender_address: declare_v2.sender_address().into(),
                        nonce: txn.nonce().into(),
                        max_fee: Fee(txn.max_fee()),
                        version: txn.version().into(),
                        transaction_hash: txn.get_hash().unwrap_or_default().into(),
                        signature: into_vec(txn.signature()),
                    };

                    TransactionWithType {
                        r#type: TransactionType::Declare,
                        transaction: Transaction::Declare(DeclareTransaction::Version2(
                            declare_txn,
                        )),
                    }
                }
                starknet_core::transactions::Transaction::DeployAccount(deploy_account) => {
                    let deploy_account_txn = DeployAccountTransaction {
                        nonce: txn.nonce().into(),
                        max_fee: Fee(txn.max_fee()),
                        version: txn.version().into(),
                        transaction_hash: txn.get_hash().unwrap_or_default().into(),
                        signature: into_vec(txn.signature()),
                        class_hash: deploy_account
                            .class_hash()
                            .map_err(ApiError::StarknetDevnetError)?
                            .into(),
                        contract_address_salt: deploy_account.contract_address_salt().into(),
                        constructor_calldata: into_vec(&deploy_account.constructor_calldata()),
                    };

                    TransactionWithType {
                        r#type: TransactionType::DeployAccount,
                        transaction: Transaction::DeployAccount(deploy_account_txn),
                    }
                }
                starknet_core::transactions::Transaction::Invoke(invoke_v1) => {
                    let invoke_txn = InvokeTransactionV1 {
                        sender_address: invoke_v1
                            .sender_address()
                            .map_err(ApiError::StarknetDevnetError)?
                            .into(),
                        nonce: txn.nonce().into(),
                        max_fee: Fee(txn.max_fee()),
                        version: txn.version().into(),
                        transaction_hash: txn.get_hash().unwrap_or_default().into(),
                        signature: into_vec(txn.signature()),
                        calldata: into_vec(invoke_v1.calldata()),
                    };

                    TransactionWithType {
                        r#type: TransactionType::Invoke,
                        transaction: Transaction::Invoke(
                            crate::api::models::transaction::InvokeTransaction::Version1(
                                invoke_txn,
                            ),
                        ),
                    }
                }
            };

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
        _block_id: BlockId,
        _contract_address: ContractAddressHex,
    ) -> RpcResult<FeltHex> {
        Err(error::ApiError::BlockNotFound)
    }
}
