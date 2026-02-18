use std::sync::Arc;

use serde::{Deserialize, Deserializer};
use starknet_rs_core::types::Felt;

use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{Calldata, Proof, ProofFacts};

fn deserialize_proof<'de, D>(deserializer: D) -> Result<Option<Proof>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(base64_str) => {
            // Decode base64 string to bytes
            let bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                base64_str.as_bytes(),
            )
            .map_err(serde::de::Error::custom)?;

            // Convert bytes to Vec<u32>
            if bytes.len() % 4 != 0 {
                return Err(serde::de::Error::custom("Proof bytes length must be a multiple of 4"));
            }

            let mut proof = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks(4) {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(chunk);
                proof.push(u32::from_be_bytes(arr));
            }

            Ok(Some(proof))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
    pub account_deployment_data: Vec<Felt>,
    #[serde(default, deserialize_with = "deserialize_proof")]
    pub proof: Option<Proof>,
    #[serde(default)]
    pub proof_facts: Option<ProofFacts>,
}

impl BroadcastedInvokeTransactionV3 {
    pub fn create_sn_api_invoke(
        &self,
        drop_proof_facts: bool,
    ) -> DevnetResult<starknet_api::transaction::InvokeTransaction> {
        let sn_api_transaction = starknet_api::transaction::InvokeTransactionV3 {
            resource_bounds: (&self.common.resource_bounds).into(),
            tip: self.common.tip,
            signature: starknet_api::transaction::fields::TransactionSignature(Arc::new(
                self.common.signature.clone(),
            )),
            nonce: starknet_api::core::Nonce(self.common.nonce),
            sender_address: self.sender_address.into(),
            calldata: starknet_api::transaction::fields::Calldata(Arc::new(self.calldata.clone())),
            nonce_data_availability_mode: self.common.nonce_data_availability_mode,
            fee_data_availability_mode: self.common.fee_data_availability_mode,
            paymaster_data: starknet_api::transaction::fields::PaymasterData(
                self.common.paymaster_data.clone(),
            ),
            account_deployment_data: starknet_api::transaction::fields::AccountDeploymentData(
                self.account_deployment_data.clone(),
            ),
            proof_facts: if drop_proof_facts {
                Vec::new()
            } else {
                self.proof_facts.clone().unwrap_or_default()
            }
            .into(),
        };

        Ok(starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction))
    }
}
