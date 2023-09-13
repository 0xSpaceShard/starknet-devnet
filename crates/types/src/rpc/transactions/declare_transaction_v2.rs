use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::transaction::DeclareV2 as SirDeclareV2;
use starknet_in_rust::SierraContractClass;

use crate::contract_address::ContractAddress;
use crate::error::{DevnetResult, Error};
use crate::felt::{
    ClassHash, CompiledClassHash, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeclareTransactionV2 {
    pub class_hash: ClassHash,
    pub compiled_class_hash: CompiledClassHash,
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub transaction_hash: TransactionHash,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV2 {
    pub fn get_max_fee(&self) -> Fee {
        self.max_fee
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

impl TryFrom<SirDeclareV2> for DeclareTransactionV2 {
    type Error = Error;
    fn try_from(value: SirDeclareV2) -> DevnetResult<Self> {
        Ok(Self {
            class_hash: value.sierra_class_hash.into(),
            compiled_class_hash: value.compiled_class_hash.into(),
            sender_address: value.sender_address.try_into()?,
            contract_class: value.sierra_contract_class,
            nonce: value.nonce.into(),
            max_fee: Fee(value.max_fee),
            version: value.version.into(),
            transaction_hash: value.hash_value.into(),
            signature: value.signature.iter().map(Felt::from).collect(),
        })
    }
}
