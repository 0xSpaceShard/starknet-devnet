use cairo_felt::Felt252;
use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix as SirTransactionHashPrefix,
};
use starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::definitions::transaction_type::TransactionType as SirTransactionType;
use starknet_in_rust::transaction::{verify_version, Declare as SirDeclare};

use crate::contract_address::ContractAddress;
use crate::contract_class::Cairo0ContractClass;
use crate::error::DevnetResult;
use crate::felt::{
    ClassHash, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};
use crate::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
use crate::rpc::transactions::BroadcastedTransactionCommon;
use crate::traits::HashProducer;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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
    pub fn compile_sir_declare(
        &self,
        class_hash: ClassHash,
        transaction_hash: TransactionHash,
    ) -> DevnetResult<SirDeclare> {
        let declare = SirDeclare {
            class_hash: class_hash.into(),
            sender_address: self.sender_address.into(),
            tx_type: SirTransactionType::Declare,
            validate_entry_point_selector: VALIDATE_DECLARE_ENTRY_POINT_SELECTOR.clone(),
            version: self.common.version.into(),
            max_fee: self.common.max_fee.0,
            signature: self.common.signature.iter().map(|felt| felt.into()).collect(),
            nonce: self.common.nonce.into(),
            hash_value: transaction_hash.into(),
            contract_class: self.contract_class.clone().try_into()?, /* ? Not present in
                                                                      * DeclareTransactionV0V1 */
            skip_execute: false,
            skip_fee_transfer: false,
            skip_validate: false,
        };

        verify_version(&declare.version, declare.max_fee, &declare.nonce, &declare.signature)?;

        Ok(declare)
    }

    pub fn compile_declare(
        &self,
        class_hash: ClassHash,
        transaction_hash: TransactionHash,
    ) -> DeclareTransactionV0V1 {
        DeclareTransactionV0V1 {
            class_hash,
            sender_address: self.sender_address,
            nonce: self.common.nonce,
            max_fee: self.common.max_fee,
            version: self.common.version,
            transaction_hash,
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
        let additional_data: Vec<Felt252> = vec![self.common.nonce.into()];
        let calldata = vec![class_hash.into()];
        // TODO: Remove when SirDeclare::new will give same hash
        Ok(calculate_transaction_hash_common(
            SirTransactionHashPrefix::Declare,
            self.common.version.into(),
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
