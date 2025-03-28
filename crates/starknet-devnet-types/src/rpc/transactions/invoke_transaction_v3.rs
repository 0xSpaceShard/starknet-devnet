use serde::Serialize;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::fields::Tip;
use starknet_types_core::felt::Felt;

use super::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use super::{BroadcastedTransactionCommonV3, ResourceBoundsWrapper};
use crate::contract_address::ContractAddress;
use crate::felt::{Calldata, Nonce, TransactionSignature, TransactionVersion};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, PartialEq, Eq),
    serde(deny_unknown_fields)
)]
pub struct InvokeTransactionV3 {
    version: TransactionVersion,
    signature: TransactionSignature,
    nonce: Nonce,
    resource_bounds: ResourceBoundsWrapper,
    tip: Tip,
    paymaster_data: Vec<Felt>,
    nonce_data_availability_mode: DataAvailabilityMode,
    fee_data_availability_mode: DataAvailabilityMode,
    account_deployment_data: Vec<Felt>,
    pub(crate) sender_address: ContractAddress,
    calldata: Calldata,
}

impl InvokeTransactionV3 {
    pub fn new(broadcasted_txn: &BroadcastedInvokeTransactionV3) -> Self {
        Self {
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
            nonce: broadcasted_txn.common.nonce,
            resource_bounds: broadcasted_txn.common.resource_bounds.clone(),
            tip: broadcasted_txn.common.tip,
            paymaster_data: broadcasted_txn.common.paymaster_data.clone(),
            nonce_data_availability_mode: broadcasted_txn.common.nonce_data_availability_mode,
            fee_data_availability_mode: broadcasted_txn.common.fee_data_availability_mode,
            sender_address: broadcasted_txn.sender_address,
            calldata: broadcasted_txn.calldata.clone(),
            account_deployment_data: broadcasted_txn.account_deployment_data.clone(),
        }
    }

    pub(crate) fn get_resource_bounds(&self) -> &ResourceBoundsWrapper {
        &self.resource_bounds
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
