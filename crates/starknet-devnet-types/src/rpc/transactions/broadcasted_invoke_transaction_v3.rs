use serde::Deserialize;
use starknet_rs_core::types::Felt;

use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
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
