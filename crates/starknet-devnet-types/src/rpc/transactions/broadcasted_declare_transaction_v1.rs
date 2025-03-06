use serde::Deserialize;
use starknet_api::transaction::fields::Fee;
use starknet_rs_core::types::Felt;

use crate::contract_address::ContractAddress;
use crate::contract_class::Cairo0ContractClass;
use crate::error::DevnetResult;
use crate::felt::{Nonce, TransactionSignature, TransactionVersion};
use crate::rpc::transactions::BroadcastedTransactionCommon;
use crate::traits::HashProducer;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_class: Cairo0ContractClass,
    pub sender_address: ContractAddress,
}

impl BroadcastedDeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        contract_class: &Cairo0ContractClass,
        version: TransactionVersion,
    ) -> Self {
        Self {
            sender_address,
            contract_class: contract_class.clone(),
            common: BroadcastedTransactionCommon {
                max_fee,
                nonce,
                version,
                signature: signature.clone(),
            },
        }
    }

    pub fn generate_class_hash(&self) -> DevnetResult<Felt> {
        self.contract_class.generate_hash()
    }
}
