use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use super::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use crate::contract_address::ContractAddress;
use crate::felt::{
    ClassHash, CompiledClassHash, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionV2 {
    pub class_hash: ClassHash,
    pub compiled_class_hash: CompiledClassHash,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV2 {
    pub fn new(broadcasted_txn: &BroadcastedDeclareTransactionV2, class_hash: ClassHash) -> Self {
        Self {
            class_hash,
            compiled_class_hash: broadcasted_txn.compiled_class_hash,
            sender_address: broadcasted_txn.sender_address,
            nonce: broadcasted_txn.common.nonce,
            max_fee: broadcasted_txn.common.max_fee,
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
        }
    }
}
