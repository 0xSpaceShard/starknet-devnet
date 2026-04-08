use std::sync::Arc;

use serde::Deserialize;
use serde::de::Deserializer;
use starknet_rs_core::types::Felt;

use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{Calldata, ProofFacts};
use crate::proof::Proof;

/// Normalize an empty `Proof` (from `""`) to `None`.
fn deserialize_optional_proof<'de, D>(deserializer: D) -> Result<Option<Proof>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Proof> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|p| !p.is_empty()))
}

/// Normalize an empty `ProofFacts` (from `[]`) to `None`.
fn deserialize_optional_proof_facts<'de, D>(deserializer: D) -> Result<Option<ProofFacts>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<ProofFacts> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|pf| !pf.is_empty()))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
    pub account_deployment_data: Vec<Felt>,
    #[serde(default, deserialize_with = "deserialize_optional_proof")]
    pub proof: Option<Proof>,
    #[serde(default, deserialize_with = "deserialize_optional_proof_facts")]
    pub proof_facts: Option<ProofFacts>,
}

impl BroadcastedInvokeTransactionV3 {
    pub fn create_sn_api_invoke(
        &self,
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
            proof_facts: self.proof_facts.clone().unwrap_or_default().into(),
        };

        Ok(starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction))
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    /// Minimal valid JSON for a `BroadcastedInvokeTransactionV3` (without proof fields).
    fn base_json() -> serde_json::Value {
        serde_json::json!({
            "version": "0x3",
            "signature": [],
            "nonce": "0x0",
            "resource_bounds": {
                "l1_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
                "l2_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
                "l1_data_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" }
            },
            "tip": "0x0",
            "paymaster_data": [],
            "nonce_data_availability_mode": "L1",
            "fee_data_availability_mode": "L1",
            "sender_address": "0x123",
            "calldata": [],
            "account_deployment_data": []
        })
    }

    fn with_proof_fields(
        mut base: serde_json::Value,
        proof: Option<serde_json::Value>,
        proof_facts: Option<serde_json::Value>,
    ) -> serde_json::Value {
        if let Some(p) = proof {
            base["proof"] = p;
        }
        if let Some(pf) = proof_facts {
            base["proof_facts"] = pf;
        }
        base
    }

    #[test]
    fn omitted_proof_fields_deserialize_as_none() {
        let json = base_json();
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        assert!(tx.proof.is_none());
        assert!(tx.proof_facts.is_none());
    }

    #[test]
    fn empty_string_proof_deserializes_as_none() {
        let json = with_proof_fields(base_json(), Some(serde_json::json!("")), None);
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        assert!(tx.proof.is_none(), "empty string proof should be normalized to None");
    }

    #[test]
    fn empty_array_proof_facts_deserializes_as_none() {
        let json = with_proof_fields(base_json(), None, Some(serde_json::json!([])));
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        assert!(tx.proof_facts.is_none(), "empty array proof_facts should be normalized to None");
    }

    #[test]
    fn both_empty_proof_and_proof_facts_deserialize_as_none() {
        let json = with_proof_fields(
            base_json(),
            Some(serde_json::json!("")),
            Some(serde_json::json!([])),
        );
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        assert!(tx.proof.is_none(), "empty string proof should be normalized to None");
        assert!(tx.proof_facts.is_none(), "empty array proof_facts should be normalized to None");
    }

    #[test]
    fn non_empty_proof_preserved() {
        let proof_bytes = vec![1u8, 2, 3, 4];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&proof_bytes);
        let json = with_proof_fields(base_json(), Some(serde_json::json!(b64)), None);
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        assert_eq!(tx.proof.unwrap().into_inner(), proof_bytes);
    }

    #[test]
    fn non_empty_proof_facts_preserved() {
        let json = with_proof_fields(base_json(), None, Some(serde_json::json!(["0x1", "0x2"])));
        let tx: BroadcastedInvokeTransactionV3 = serde_json::from_value(json).unwrap();
        let pf = tx.proof_facts.unwrap();
        assert_eq!(pf.len(), 2);
        assert_eq!(pf[0], Felt::ONE);
        assert_eq!(pf[1], Felt::TWO);
    }
}
