use cairo_felt::Felt252;
use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::definitions::transaction_type::TransactionType as SirTransactionType;
use starknet_in_rust::transaction::Declare as SirDeclare;

use crate::contract_address::ContractAddress;
use crate::contract_class::DeprecatedContractClass;
use crate::error::{DevnetResult, Error};
use crate::felt::{
    ClassHash, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeclareTransactionV0V1 {
    pub class_hash: ClassHash,
    pub sender_address: ContractAddress,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub transaction_hash: TransactionHash,
    pub signature: TransactionSignature,
}

impl DeclareTransactionV0V1 {
    fn compile_sir_declare(
        &self,
        contract_class: DeprecatedContractClass,
    ) -> DevnetResult<SirDeclare> {
        Ok(SirDeclare {
            class_hash: self.class_hash.clone().into(),
            sender_address: self.sender_address.into(),
            tx_type: SirTransactionType::Declare,
            validate_entry_point_selector: VALIDATE_DECLARE_ENTRY_POINT_SELECTOR.clone(),
            version: self.version.into(),
            max_fee: self.max_fee.0,
            signature: self.signature.iter().map(|felt| felt.into()).collect(),
            nonce: self.nonce.into(),
            hash_value: Felt252::default(),
            contract_class: contract_class.try_into()?, /* ? Not present in
                                                         * DeclareTransactionV0V1 */
            skip_execute: false,
            skip_fee_transfer: false,
            skip_validate: false,
        })
    }

    pub fn max_fee(&self) -> Fee {
        self.max_fee
    }

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
