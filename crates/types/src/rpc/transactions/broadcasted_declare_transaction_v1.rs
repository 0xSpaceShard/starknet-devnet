use serde::{Deserialize, Serialize};

use crate::contract_address::ContractAddress;
use crate::contract_class::DeprecatedContractClass;
use crate::error::DevnetResult;
use crate::felt::{ClassHash, Felt, TransactionHash};
use crate::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
use crate::rpc::transactions::BroadcastedTransactionCommon;
use crate::traits::HashProducer;
use cairo_felt::Felt252;
use starknet_api::class_hash;
use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix as SirTransactionHashPrefix,
};
use starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::definitions::transaction_type::TransactionType as SirTransactionType;
use starknet_in_rust::transaction::Declare as SirDeclare;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeclareTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_class: DeprecatedContractClass,
    pub sender_address: ContractAddress,
}

impl BroadcastedDeclareTransactionV1 {
    pub fn compile_sir_declare(&self, class_hash: &ClassHash) -> DevnetResult<SirDeclare> {
        Ok(SirDeclare {
            class_hash: class_hash.clone().into(),
            sender_address: self.sender_address.into(),
            tx_type: SirTransactionType::Declare,
            validate_entry_point_selector: VALIDATE_DECLARE_ENTRY_POINT_SELECTOR.clone(),
            version: self.common.version.into(),
            max_fee: self.common.max_fee.0,
            signature: self.common.signature.iter().map(|felt| felt.into()).collect(),
            nonce: self.common.nonce.into(),
            hash_value: Felt252::default(),
            contract_class: self.contract_class.clone().try_into()?, // ? Not present in DeclareTransactionV0V1
            skip_execute: false,
            skip_fee_transfer: false,
            skip_validate: false,
        })
    }

    pub fn compile_declare(
        &self,
        class_hash: &ClassHash,
        transaction_hash: &TransactionHash,
    ) -> DeclareTransactionV0V1 {
        DeclareTransactionV0V1 {
            class_hash: class_hash.clone(),
            sender_address: self.sender_address,
            nonce: self.common.nonce,
            max_fee: self.common.max_fee,
            version: self.common.version,
            transaction_hash: transaction_hash.clone(),
            signature: self.common.signature.clone(),
        }
    }

    pub fn generate_class_hash(&self) -> DevnetResult<Felt> {
        self.contract_class.generate_hash()
    }

    // TODO: Maybe move outside
    pub fn calculate_transaction_hash(
        &self,
        chain_id: &Felt,
        class_hash: &ClassHash,
    ) -> DevnetResult<ClassHash> {
        let additional_data: Vec<Felt252> = vec![self.common.nonce.clone().into()];
        let calldata = vec![class_hash.into()];
        // TODO: SirDeclare::new uses same logic, check if can replace
        Ok(calculate_transaction_hash_common(
            SirTransactionHashPrefix::Declare,
            self.common.version.clone().into(),
            &self.sender_address.into(),
            Felt252::from(0),
            &calldata,
            self.common.max_fee.0,
            chain_id.into(),
            &additional_data,
        )?
        .into())
    }
}
