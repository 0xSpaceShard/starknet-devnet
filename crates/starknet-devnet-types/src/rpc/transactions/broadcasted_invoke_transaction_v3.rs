use std::sync::Arc;

use serde::Deserialize;
use starknet_rs_core::types::Felt;

use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::Calldata;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
    pub account_deployment_data: Vec<Felt>,
}

impl BroadcastedInvokeTransactionV3 {
    pub fn create_sn_api_invoke(
        &self,
    ) -> DevnetResult<starknet_api::transaction::InvokeTransaction> {
        let sn_api_transaction = starknet_api::transaction::InvokeTransactionV3 {
            resource_bounds: (&self.common.resource_bounds).into(),
            tip: self.common.tip,
            signature: starknet_api::transaction::fields::TransactionSignature(
                self.common.signature.clone(),
            ),
            nonce: starknet_api::core::Nonce(self.common.nonce),
            sender_address: self.sender_address.try_into()?,
            calldata: starknet_api::transaction::fields::Calldata(Arc::new(self.calldata.clone())),
            nonce_data_availability_mode: self.common.nonce_data_availability_mode,
            fee_data_availability_mode: self.common.fee_data_availability_mode,
            paymaster_data: starknet_api::transaction::fields::PaymasterData(
                self.common.paymaster_data.clone(),
            ),
            account_deployment_data: starknet_api::transaction::fields::AccountDeploymentData(
                self.account_deployment_data.clone(),
            ),
        };

        Ok(starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction))
    }
}
