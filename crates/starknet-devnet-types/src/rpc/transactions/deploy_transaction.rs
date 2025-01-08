use serde::{Deserialize, Serialize};

use crate::felt::{Calldata, ClassHash, ContractAddressSalt, TransactionVersion};

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(Deserialize), serde(deny_unknown_fields))]
pub struct DeployTransaction {
    pub version: TransactionVersion,
    pub class_hash: ClassHash,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
}
