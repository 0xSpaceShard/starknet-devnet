use serde::Serialize;
use starknet_api::transaction::Fee;

use super::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use crate::contract_address::ContractAddress;
use crate::felt::{ClassHash, Nonce, TransactionSignature, TransactionVersion};
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, PartialEq, Eq),
    serde(deny_unknown_fields)
)]
pub struct DeclareTransactionV0V1 {
    pub class_hash: ClassHash,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV0V1 {
    pub fn new(broadcasted_txn: &BroadcastedDeclareTransactionV1, class_hash: ClassHash) -> Self {
        Self {
            class_hash,
            sender_address: broadcasted_txn.sender_address,
            nonce: broadcasted_txn.common.nonce,
            max_fee: broadcasted_txn.common.max_fee,
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
        }
    }
}
