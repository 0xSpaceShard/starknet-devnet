use serde::Serialize;

use crate::felt::{Calldata, ClassHash, ContractAddressSalt, TransactionVersion};

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, PartialEq, Eq),
    serde(deny_unknown_fields)
)]
pub struct DeployTransaction {
    pub version: TransactionVersion,
    pub class_hash: ClassHash,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
}
