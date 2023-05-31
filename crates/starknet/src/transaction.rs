use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use starknet_api::transaction::TransactionHash;

use crate::traits::HashIdentified;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    /// Transaction has not been received yet (i.e. not written to storage)
    NotReceived,
    /// Transaction was received by the sequenced
    Received,
    /// Transaction passed teh validation and entered the pending block
    Pending,
    /// The transaction failed validation and was skipped (applies both to a
    /// pending and actual created block)
    Rejected,
    /// Transaction passed teh validation and entered a created block
    AcceptedOnL2,
    /// Transaction was accepted on-chain
    AcceptedOnL1,
}

#[derive(Debug, Clone)]
pub(crate) struct StarknetTransaction {
    status: TransactionStatus,
    inner: starknet_api::transaction::Transaction,
}

#[derive(Debug, Default)]
pub(crate) struct StarknetTransactions {
    transactions: HashMap<TransactionHash, StarknetTransaction>,
}

impl HashIdentified for StarknetTransactions {
    type Element = Option<StarknetTransaction>;
    type Hash = TransactionHash;

    fn get_by_hash(&self, hash: Self::Hash) -> Self::Element {
        self.transactions.get(&hash).cloned()
    }
}
