pub mod declare_transaction;
pub mod declare_transaction_v2;
pub mod deploy_account_transaction;
pub mod invoke_transaction;

use std::collections::HashMap;

use starknet_api::block::BlockNumber;
use starknet_api::transaction::Fee;
use starknet_in_rust::execution::{CallInfo, Event, TransactionExecutionInfo};
use starknet_in_rust::transaction::error::TransactionError;
use starknet_rs_core::types::TransactionStatus;
use starknet_types::felt::{BlockHash, Felt, TransactionHash};
use starknet_types::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1 as RpcDeclareTransactionV0V1;
use starknet_types::rpc::transactions::declare_transaction_v2::DeclareTransactionV2 as RpcDeclareTransactionV2;
use starknet_types::rpc::transactions::deploy_account_transaction::DeployAccountTransaction as RpcDeployAccountTransaction;
use starknet_types::rpc::transactions::invoke_transaction_v1::InvokeTransactionV1 as RpcInvokeTransactionV1;
use starknet_types::rpc::transactions::{
    DeclareTransaction as RpcDeclareTransaction, Transaction as RpcTransaction,
    TransactionType as RpcTransactionType, TransactionWithType as RpcTransactionWithType,
};

use self::declare_transaction::DeclareTransactionV1;
use self::declare_transaction_v2::DeclareTransactionV2;
use self::deploy_account_transaction::DeployAccountTransaction;
use self::invoke_transaction::InvokeTransactionV1;
use crate::error::{DevnetResult, Error};
use crate::traits::{HashIdentified, HashIdentifiedMut};

#[derive(Default)]
pub struct StarknetTransactions(HashMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
    }

    pub fn get(&self, transaction_hash: &TransactionHash) -> Option<&StarknetTransaction> {
        self.0.get(transaction_hash)
    }
}

impl HashIdentifiedMut for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash_mut(&mut self, hash: &Self::Hash) -> Option<&mut StarknetTransaction> {
        self.0.get_mut(hash)
    }
}

impl HashIdentified for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash(&self, hash: Self::Hash) -> Option<&StarknetTransaction> {
        self.0.get(&hash)
    }
}

#[allow(unused)]
pub struct StarknetTransaction {
    pub(crate) status: TransactionStatus,
    pub inner: RpcTransactionWithType,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    pub(crate) execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    pub(crate) execution_error: Option<TransactionError>,
}

impl StarknetTransaction {
    pub fn create_rejected(
        transaction: RpcTransactionWithType,
        execution_error: TransactionError,
    ) -> Self {
        Self {
            status: TransactionStatus::Rejected,
            inner: transaction,
            execution_info: None,
            execution_error: Some(execution_error),
            block_hash: None,
            block_number: None,
        }
    }

    // TODO: pass by reference
    pub fn create_successful(
        transaction: RpcTransactionWithType,
        execution_info: TransactionExecutionInfo,
    ) -> Self {
        Self {
            status: TransactionStatus::Pending,
            inner: transaction,
            execution_info: Some(execution_info),
            execution_error: None,
            block_hash: None,
            block_number: None,
        }
    }

    pub fn get_events(&self) -> DevnetResult<Vec<Event>> {
        let mut result = Vec::<Event>::new();

        fn events_from_call_info(call_info: Option<&CallInfo>) -> DevnetResult<Vec<Event>> {
            if let Some(call_info) = call_info {
                call_info.get_sorted_events().map_err(crate::error::Error::from)
            } else {
                Ok(Vec::<Event>::new())
            }
        }

        if let Some(execution_info) = &self.execution_info {
            result.extend(events_from_call_info(execution_info.validate_info.as_ref())?);
            result.extend(events_from_call_info(execution_info.call_info.as_ref())?);
            result.extend(events_from_call_info(execution_info.fee_transfer_info.as_ref())?);
        }

        Ok(result)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Transaction {
    Declare(Box<DeclareTransactionV1>),
    DeclareV2(Box<DeclareTransactionV2>),
    DeployAccount(Box<DeployAccountTransaction>),
    Invoke(Box<InvokeTransactionV1>),
}

impl Transaction {
    pub fn get_hash(&self) -> TransactionHash {
        match self {
            Transaction::Declare(tx) => tx.transaction_hash,
            Transaction::DeclareV2(tx) => tx.transaction_hash,
            Transaction::DeployAccount(tx) => tx.inner.hash_value().clone().into(),
            Transaction::Invoke(tx) => tx.inner.hash_value().clone().into(),
        }
    }

    pub fn chain_id(&self) -> &Felt {
        match self {
            Transaction::Declare(txn) => &txn.chain_id,
            Transaction::DeclareV2(txn) => &txn.chain_id,
            Transaction::DeployAccount(txn) => &txn.chain_id,
            Transaction::Invoke(txn) => &txn.chain_id,
        }
    }

    pub fn max_fee(&self) -> u128 {
        match self {
            Transaction::Declare(txn) => txn.max_fee,
            Transaction::DeclareV2(txn) => txn.max_fee,
            Transaction::DeployAccount(txn) => txn.max_fee,
            Transaction::Invoke(txn) => txn.max_fee,
        }
    }

    pub fn signature(&self) -> &Vec<Felt> {
        match self {
            Transaction::Declare(txn) => &txn.signature,
            Transaction::DeclareV2(txn) => &txn.signature,
            Transaction::DeployAccount(txn) => &txn.signature,
            Transaction::Invoke(txn) => &txn.signature,
        }
    }

    pub fn nonce(&self) -> &Felt {
        match self {
            Transaction::Declare(txn) => &txn.nonce,
            Transaction::DeclareV2(txn) => &txn.nonce,
            Transaction::DeployAccount(txn) => &txn.nonce,
            Transaction::Invoke(txn) => &txn.nonce,
        }
    }

    pub fn version(&self) -> &Felt {
        match self {
            Transaction::Declare(txn) => &txn.version,
            Transaction::DeclareV2(txn) => &txn.version,
            Transaction::DeployAccount(txn) => &txn.version,
            Transaction::Invoke(txn) => &txn.version,
        }
    }
}

impl TryFrom<&Transaction> for RpcTransactionWithType {
    type Error = Error;
    fn try_from(txn: &Transaction) -> DevnetResult<Self> {
        let transaction_with_type = match txn {
            Transaction::Declare(declare_v1) => {
                let declare_txn = RpcDeclareTransactionV0V1 {
                    class_hash: *declare_v1.class_hash(),
                    sender_address: *declare_v1.sender_address(),
                    nonce: *txn.nonce(),
                    max_fee: Fee(txn.max_fee()),
                    version: *txn.version(),
                    transaction_hash: txn.get_hash(),
                    signature: txn.signature().to_vec(),
                };
                RpcTransactionWithType {
                    r#type: RpcTransactionType::Declare,
                    transaction: RpcTransaction::Declare(RpcDeclareTransaction::Version1(
                        declare_txn,
                    )),
                }
            }
            Transaction::DeclareV2(declare_v2) => {
                let declare_txn = RpcDeclareTransactionV2 {
                    class_hash: *declare_v2.class_hash(),
                    compiled_class_hash: *declare_v2.compiled_class_hash(),
                    sender_address: *declare_v2.sender_address(),
                    nonce: *txn.nonce(),
                    max_fee: Fee(txn.max_fee()),
                    version: *txn.version(),
                    transaction_hash: txn.get_hash(),
                    signature: txn.signature().to_vec(),
                };

                RpcTransactionWithType {
                    r#type: RpcTransactionType::Declare,
                    transaction: RpcTransaction::Declare(RpcDeclareTransaction::Version2(
                        declare_txn,
                    )),
                }
            }
            Transaction::DeployAccount(deploy_account) => {
                let deploy_account_txn = RpcDeployAccountTransaction {
                    nonce: *txn.nonce(),
                    max_fee: Fee(txn.max_fee()),
                    version: *txn.version(),
                    transaction_hash: txn.get_hash(),
                    signature: txn.signature().to_vec(),
                    class_hash: deploy_account.class_hash()?,
                    contract_address_salt: deploy_account.contract_address_salt(),
                    constructor_calldata: deploy_account.constructor_calldata(),
                };

                RpcTransactionWithType {
                    r#type: RpcTransactionType::DeployAccount,
                    transaction: RpcTransaction::DeployAccount(deploy_account_txn),
                }
            }
            Transaction::Invoke(invoke_v1) => {
                let invoke_txn = RpcInvokeTransactionV1 {
                    sender_address: invoke_v1.sender_address()?,
                    nonce: *txn.nonce(),
                    max_fee: Fee(txn.max_fee()),
                    version: *txn.version(),
                    transaction_hash: txn.get_hash(),
                    signature: txn.signature().to_vec(),
                    calldata: invoke_v1.calldata().to_vec(),
                };

                RpcTransactionWithType {
                    r#type: RpcTransactionType::Invoke,
                    transaction: RpcTransaction::Invoke(
                        starknet_types::rpc::transactions::InvokeTransaction::Version1(invoke_txn),
                    ),
                }
            }
        };

        Ok(transaction_with_type)
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::TransactionExecutionInfo;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::rpc::transactions::{
        DeclareTransaction, Transaction, TransactionType, TransactionWithType,
    };
    use starknet_types::traits::HashProducer;

    use super::{StarknetTransaction, StarknetTransactions};
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::dummy_declare_transaction_v1;

    #[test]
    fn get_transaction_by_hash() {
        let declare_transaction = dummy_declare_transaction_v1();
        let hash = declare_transaction.generate_hash().unwrap();
        let tx_with_type = TransactionWithType {
            r#type: TransactionType::Declare,
            transaction: Transaction::Declare(DeclareTransaction::Version1(declare_transaction)),
        };

        let sn_tx = StarknetTransaction::create_successful(
            tx_with_type.clone(),
            TransactionExecutionInfo::default(),
        );
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            &hash,
            StarknetTransaction::create_successful(
                tx_with_type,
                TransactionExecutionInfo::default(),
            ),
        );

        let extracted_tran = sn_txs.get_by_hash_mut(&hash).unwrap();

        assert_eq!(sn_tx.block_hash, extracted_tran.block_hash);
        assert_eq!(sn_tx.block_number, extracted_tran.block_number);
        assert!(sn_tx.inner == extracted_tran.inner);
        assert_eq!(sn_tx.status, extracted_tran.status);
        assert_eq!(sn_tx.execution_error.is_some(), extracted_tran.execution_error.is_some());
        assert_eq!(sn_tx.execution_info.is_some(), extracted_tran.execution_info.is_some());
    }

    #[test]
    fn check_correct_rejected_transaction_creation() {
        let tx = TransactionWithType {
            r#type: TransactionType::Declare,
            transaction: Transaction::Declare(DeclareTransaction::Version1(
                dummy_declare_transaction_v1(),
            )),
        };
        check_correct_transaction_properties(tx, false);
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        let tx = TransactionWithType {
            r#type: TransactionType::Declare,
            transaction: Transaction::Declare(DeclareTransaction::Version1(
                dummy_declare_transaction_v1(),
            )),
        };
        check_correct_transaction_properties(tx, true);
    }

    fn check_correct_transaction_properties(tran: TransactionWithType, is_success: bool) {
        let sn_tran = if is_success {
            StarknetTransaction::create_successful(
                tran.clone(),
                TransactionExecutionInfo::default(),
            )
        } else {
            StarknetTransaction::create_rejected(
                tran.clone(),
                starknet_in_rust::transaction::error::TransactionError::AttempToUseNoneCodeAddress,
            )
        };

        if is_success {
            assert!(sn_tran.status == TransactionStatus::Pending);
        } else {
            assert!(sn_tran.status == TransactionStatus::Rejected);
        }

        assert_eq!(sn_tran.execution_info.is_some(), is_success);
        assert_eq!(sn_tran.execution_error.is_none(), is_success);
        assert!(sn_tran.block_hash.is_none());
        assert!(sn_tran.block_number.is_none());
        assert!(sn_tran.inner == tran);
    }
}
