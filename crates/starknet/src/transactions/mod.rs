pub(crate) mod declare_transaction;

use std::collections::HashMap;

use starknet_in_rust::{execution::TransactionExecutionInfo, transaction::error::TransactionError};
use starknet_rs_core::types::TransactionStatus;
use starknet_types::{felt::TransactionHash, error::Error};

use crate::traits::HashIdentified;

use self::declare_transaction::DeclareTransactionV1;

#[derive(Default)]
pub struct StarknetTransactions(HashMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
    }
}

impl HashIdentified for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash(&self, hash: Self::Hash) -> starknet_types::DevnetResult<Option<&Self::Element>> {
        let result = self.0.get(&hash);

        Ok(result)
    }
}

pub struct StarknetTransaction {
    status: TransactionStatus,
    inner: Transaction,
    execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    execution_error: Option<TransactionError>,
}

impl StarknetTransaction {
    pub fn create_pending(
        transaction: Transaction,
        execution_info: TransactionExecutionInfo,
    ) -> Self {
        Self {
            status: TransactionStatus::Pending,
            inner: transaction,
            execution_info: Some(execution_info),
            execution_error: None,
        }
    }

    pub fn create_rejected(transaction: Transaction, execution_error: TransactionError) -> Self {
        Self {
            status: TransactionStatus::Rejected,
            inner: transaction,
            execution_info: None,
            execution_error: Some(execution_error),
        }
    }
}

#[derive(Clone)]
pub enum Transaction {
    Declare(DeclareTransactionV1),
}
