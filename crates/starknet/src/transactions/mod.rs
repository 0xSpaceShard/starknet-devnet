pub(crate) mod declare_transaction;

use std::collections::HashMap;

use starknet_api::block::BlockNumber;
use starknet_in_rust::{execution::TransactionExecutionInfo, transaction::error::TransactionError};
use starknet_rs_core::types::TransactionStatus;
use starknet_types::felt::{BlockHash, TransactionHash};

use crate::traits::HashIdentifiedMut;

use self::declare_transaction::DeclareTransactionV1;

#[derive(Default)]
pub struct StarknetTransactions(HashMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
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
    inner: Transaction,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    execution_error: Option<TransactionError>,
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
    Declare(DeclareTransactionV1),
}

impl Transaction {
    pub(crate) fn get_hash(&self) -> Option<TransactionHash> {
        match self {
            Transaction::Declare(tx) => tx.transaction_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::TransactionExecutionInfo;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::traits::HashProducer;

    use crate::{traits::HashIdentifiedMut, utils::test_utils::dummy_declare_transaction_v1};

    use super::{StarknetTransaction, StarknetTransactions, Transaction};

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
