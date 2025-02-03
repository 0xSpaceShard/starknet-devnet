use serde::Deserialize;

use super::BroadcastedTransactionCommonV3;
use crate::felt::{Calldata, ClassHash, ContractAddressSalt};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}
