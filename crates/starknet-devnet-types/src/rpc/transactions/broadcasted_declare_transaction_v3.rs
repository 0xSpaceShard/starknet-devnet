use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
use serde::Deserialize;
use starknet_rs_core::types::Felt;

use super::{BroadcastedDeclareTransaction, BroadcastedTransactionCommonV3};
use crate::contract_address::ContractAddress;
use crate::felt::CompiledClassHash;
use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    #[serde(deserialize_with = "deserialize_to_sierra_contract_class")]
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddress,
    pub compiled_class_hash: CompiledClassHash,
    pub account_deployment_data: Vec<Felt>,
}

impl From<BroadcastedDeclareTransactionV3> for BroadcastedDeclareTransaction {
    fn from(value: BroadcastedDeclareTransactionV3) -> Self {
        Self::V3(Box::new(value))
    }
}

// This file used to contain a test which asserts tx hash calculation. But this is no longer
// Devnet's responsibility, so there are no such tests.
