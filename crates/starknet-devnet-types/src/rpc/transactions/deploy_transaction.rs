use serde::{Deserialize, Serialize};

use crate::felt::{Calldata, ClassHash, ContractAddressSalt, TransactionVersion};

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployTransaction {
    pub version: TransactionVersion,
    pub class_hash: ClassHash,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
}
