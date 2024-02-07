use serde::{Deserialize, Serialize};
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::Tip;

use super::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use super::ResourceBoundsWrapper;
use crate::contract_address::ContractAddress;
use crate::felt::{
    ClassHash, CompiledClassHash, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionV3 {
    version: TransactionVersion,
    signature: TransactionSignature,
    nonce: Nonce,
    resource_bounds: ResourceBoundsWrapper,
    tip: Tip,
    paymaster_data: Vec<Felt>,
    nonce_data_availability_mode: DataAvailabilityMode,
    fee_data_availability_mode: DataAvailabilityMode,
    sender_address: ContractAddress,
    compiled_class_hash: CompiledClassHash,
    class_hash: ClassHash,
    account_deployment_data: Vec<Felt>,
    transaction_hash: TransactionHash,
}

impl DeclareTransactionV3 {
    pub fn new(
        broadcasted_txn: BroadcastedDeclareTransactionV3,
        class_hash: ClassHash,
        transaction_hash: TransactionHash,
    ) -> Self {
        Self {
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature,
            nonce: broadcasted_txn.common.nonce,
            resource_bounds: broadcasted_txn.common.resource_bounds,
            tip: broadcasted_txn.common.tip,
            paymaster_data: broadcasted_txn.common.paymaster_data,
            nonce_data_availability_mode: broadcasted_txn.common.nonce_data_availability_mode,
            fee_data_availability_mode: broadcasted_txn.common.fee_data_availability_mode,
            sender_address: broadcasted_txn.sender_address,
            account_deployment_data: broadcasted_txn.account_deployment_data,
            transaction_hash,
            compiled_class_hash: broadcasted_txn.compiled_class_hash,
            class_hash,
        }
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }

    pub fn get_class_hash(&self) -> &ClassHash {
        &self.class_hash
    }
}
