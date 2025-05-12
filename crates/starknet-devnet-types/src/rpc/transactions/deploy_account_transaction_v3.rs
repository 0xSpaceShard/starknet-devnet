use serde::Serialize;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::fields::Tip;
use starknet_types_core::felt::Felt;

use super::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use super::{BroadcastedTransactionCommonV3, ResourceBoundsWrapper};
use crate::contract_address::ContractAddress;
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Nonce, TransactionSignature, TransactionVersion,
};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, PartialEq, Eq),
    serde(deny_unknown_fields)
)]
pub struct DeployAccountTransactionV3 {
    version: TransactionVersion,
    signature: TransactionSignature,
    nonce: Nonce,
    resource_bounds: ResourceBoundsWrapper,
    tip: Tip,
    paymaster_data: Vec<Felt>,
    nonce_data_availability_mode: DataAvailabilityMode,
    fee_data_availability_mode: DataAvailabilityMode,
    contract_address_salt: ContractAddressSalt,
    constructor_calldata: Calldata,
    class_hash: ClassHash,
    #[serde(skip)]
    contract_address: ContractAddress,
}

impl DeployAccountTransactionV3 {
    pub fn new(
        broadcasted_txn: &BroadcastedDeployAccountTransactionV3,
        contract_address: ContractAddress,
    ) -> Self {
        Self {
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
            nonce: broadcasted_txn.common.nonce,
            resource_bounds: broadcasted_txn.common.resource_bounds.clone(),
            tip: broadcasted_txn.common.tip,
            paymaster_data: broadcasted_txn.common.paymaster_data.clone(),
            nonce_data_availability_mode: broadcasted_txn.common.nonce_data_availability_mode,
            fee_data_availability_mode: broadcasted_txn.common.fee_data_availability_mode,
            contract_address_salt: broadcasted_txn.contract_address_salt,
            constructor_calldata: broadcasted_txn.constructor_calldata.clone(),
            class_hash: broadcasted_txn.class_hash,
            contract_address,
        }
    }

    pub fn get_contract_address(&self) -> &ContractAddress {
        &self.contract_address
    }

    pub(crate) fn get_resource_bounds(&self) -> &ResourceBoundsWrapper {
        &self.resource_bounds
    }
}

impl From<DeployAccountTransactionV3> for BroadcastedDeployAccountTransactionV3 {
    fn from(value: DeployAccountTransactionV3) -> Self {
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
            contract_address_salt: value.contract_address_salt,
            constructor_calldata: value.constructor_calldata,
            class_hash: value.class_hash,
        }
    }
}
