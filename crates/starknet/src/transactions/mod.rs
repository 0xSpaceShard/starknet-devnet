pub mod declare_transaction;
pub mod declare_transaction_v2;
pub mod deploy_account_transaction;
pub mod invoke_transaction;

use std::collections::HashMap;

use starknet_api::block::BlockNumber;
use starknet_in_rust::execution::TransactionExecutionInfo;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_rs_core::types::TransactionStatus;

use self::declare_transaction::DeclareTransactionV1;
use self::declare_transaction_v2::DeclareTransactionV2;
use self::deploy_account_transaction::DeployAccountTransaction;
use self::invoke_transaction::InvokeTransactionV1;
use crate::traits::HashIdentifiedMut;
use starknet_types::felt::{BlockHash, Felt, TransactionHash};
use starknet_types::starknet_api::transaction::{EthAddress, Fee};

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

#[allow(unused)]
pub struct StarknetTransaction {
    pub(crate) status: TransactionStatus,
    pub inner: Transaction,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    pub(crate) execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    pub(crate) execution_error: Option<TransactionError>,
}

impl StarknetTransaction {
    pub fn create_rejected(transaction: Transaction, execution_error: TransactionError) -> Self {
        Self {
            status: TransactionStatus::Rejected,
            inner: transaction,
            execution_info: None,
            execution_error: Some(execution_error),
            block_hash: None,
            block_number: None,
        }
    }

    pub fn create_successful(
        transaction: Transaction,
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
}

#[derive(Clone, PartialEq, Eq)]
pub enum Transaction {
    Declare(Box<DeclareTransactionV1>),
    DeclareV2(Box<DeclareTransactionV2>),
    DeployAccount(Box<DeployAccountTransaction>),
    Invoke(Box<InvokeTransactionV1>),
}

impl Transaction {
    pub fn get_hash(&self) -> Option<TransactionHash> {
        match self {
            Transaction::Declare(tx) => tx.transaction_hash,
            Transaction::DeclareV2(tx) => tx.transaction_hash,
            Transaction::DeployAccount(tx) => Some(tx.inner.hash_value().clone().into()),
            Transaction::Invoke(tx) => Some(tx.inner.hash_value().clone().into()),
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

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::TransactionExecutionInfo;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::traits::HashProducer;

    use super::{StarknetTransaction, StarknetTransactions, Transaction};
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::dummy_declare_transaction_v1;

    #[test]
    fn get_transaction_by_hash() {
        let hash = dummy_declare_transaction_v1().generate_hash().unwrap();
        let sn_tx = StarknetTransaction::create_successful(
            Transaction::Declare(dummy_declare_transaction_v1()),
            TransactionExecutionInfo::default(),
        );
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            &hash,
            StarknetTransaction::create_successful(
                Transaction::Declare(dummy_declare_transaction_v1()),
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
        check_correct_transaction_properties(
            Transaction::Declare(dummy_declare_transaction_v1()),
            false,
        );
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        check_correct_transaction_properties(
            Transaction::Declare(dummy_declare_transaction_v1()),
            true,
        );
    }

    fn check_correct_transaction_properties(tran: Transaction, is_success: bool) {
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
