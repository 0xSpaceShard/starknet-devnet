use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_api::deprecated_contract_class::ContractClass as DeprecatedContractClass;
use starknet_rs_core::types::Felt;

use crate::error::{DevnetResult, Error, JsonError};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cairo0ContractClass {
    Rpc(DeprecatedContractClass),
}

impl Cairo0ContractClass {
    pub fn from_json_str(s: &str) -> DevnetResult<Self> {
        Ok(serde_json::from_str(s).map_err(JsonError::SerdeJsonError)?)
    }
}

impl Serialize for Cairo0ContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Cairo0ContractClass::Rpc(contract) => contract.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Cairo0ContractClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Cairo0ContractClass::Rpc(DeprecatedContractClass::deserialize(deserializer)?))
    }
}

impl From<DeprecatedContractClass> for Cairo0ContractClass {
    fn from(value: DeprecatedContractClass) -> Self {
        Cairo0ContractClass::Rpc(value)
    }
}

impl HashProducer for Cairo0ContractClass {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        match self {
            Cairo0ContractClass::Rpc(_) => todo!("copy from blockifier"),
        }
    }
}

impl From<Cairo0ContractClass> for starknet_api::deprecated_contract_class::ContractClass {
    fn from(value: Cairo0ContractClass) -> Self {
        match value {
            Cairo0ContractClass::Rpc(contract_class) => contract_class,
        }
    }
}
