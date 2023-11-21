use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use crate::contract_address::ContractAddress;
use crate::contract_class::Cairo0ContractClass;
use crate::error::{DevnetResult, Error};
use crate::felt::{
    ClassHash, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionV0V1 {
    pub class_hash: ClassHash,
    // TODO: in spec RPC response the contract class is missing
    pub contract_class: Cairo0ContractClass,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub transaction_hash: TransactionHash,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV0V1 {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

impl HashProducer for DeclareTransactionV0V1 {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(self.transaction_hash)
    }
}
