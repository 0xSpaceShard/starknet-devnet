use cairo_lang_starknet::contract_class::ContractClass as SierraContractClass;
use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use crate::contract_address::ContractAddress;
use crate::felt::{
    ClassHash, CompiledClassHash, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionV2 {
    pub class_hash: ClassHash,
    pub compiled_class_hash: CompiledClassHash,
    // TODO: in spec RPC response the contract class is missing
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub transaction_hash: TransactionHash,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV2 {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}
