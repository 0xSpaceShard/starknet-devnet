use serde::{Deserialize, Serialize};
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::{ResourceBoundsMapping, Tip};

use super::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::felt::{
    Calldata, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeTransactionV3 {
    version: TransactionVersion,
    signature: TransactionSignature,
    nonce: Nonce,
    resource_bounds: ResourceBoundsMapping,
    tip: Tip,
    paymaster_data: Vec<Felt>,
    nonce_data_availability_mode: DataAvailabilityMode,
    fee_data_availability_mode: DataAvailabilityMode,
    account_deployment_data: Vec<Felt>,
    sender_address: ContractAddress,
    calldata: Calldata,
    transaction_hash: TransactionHash,
}

impl InvokeTransactionV3 {
    pub fn new(
        broadcasted_txn: BroadcastedInvokeTransactionV3,
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
            calldata: broadcasted_txn.calldata,
            account_deployment_data: broadcasted_txn.account_deployment_data,
            transaction_hash,
        }
    }
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

impl From<InvokeTransactionV3> for BroadcastedInvokeTransactionV3 {
    fn from(value: InvokeTransactionV3) -> Self {
        Self {
            common: BroadcastedTransactionCommonV3 {
                version: value.version,
                signature: value.signature,
                nonce: value.nonce,
                resource_bounds: value.resource_bounds,
                tip: value.tip,
                paymaster_data: value.paymaster_data,
                nonce_data_availability_mode: value.nonce_data_availability_mode,
                fee_data_availability_mode: value.fee_data_availability_mode,
            },
            sender_address: value.sender_address,
            calldata: value.calldata,
            account_deployment_data: value.account_deployment_data,
        }
    }
}
