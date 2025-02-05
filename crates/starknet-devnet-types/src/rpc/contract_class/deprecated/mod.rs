use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_rs_core::types::{CompressedLegacyContractClass, Felt};

use crate::contract_class::deprecated::json_contract_class::Cairo0Json;
use crate::contract_class::deprecated::rpc_contract_class::DeprecatedContractClass;
use crate::error::{DevnetResult, Error};
use crate::traits::HashProducer;

pub mod abi_entry;
pub mod json_contract_class;
pub mod rpc_contract_class;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cairo0ContractClass {
    // TODO: remove once starknet_api raised
    RawJson(Cairo0Json),
    Rpc(DeprecatedContractClass),
}

impl Serialize for Cairo0ContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Cairo0ContractClass::RawJson(contract_json) => contract_json.serialize(serializer),
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

impl From<Cairo0Json> for Cairo0ContractClass {
    fn from(value: Cairo0Json) -> Self {
        Cairo0ContractClass::RawJson(value)
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
            Cairo0ContractClass::RawJson(contract_json) => Ok(contract_json.generate_hash()?),
            Cairo0ContractClass::Rpc(contract) => Ok(contract.generate_hash()?),
        }
    }
}

impl TryInto<CompressedLegacyContractClass> for Cairo0ContractClass {
    type Error = Error;
    fn try_into(self) -> Result<CompressedLegacyContractClass, Self::Error> {
        match self {
            Cairo0ContractClass::Rpc(contract_class) => contract_class.try_into(),
            Cairo0ContractClass::RawJson(contract_class) => contract_class.try_into(),
        }
    }
}

impl TryFrom<Cairo0ContractClass> for starknet_api::deprecated_contract_class::ContractClass {
    type Error = Error;
    fn try_from(value: Cairo0ContractClass) -> Result<Self, Self::Error> {
        match value {
            Cairo0ContractClass::RawJson(contract_class) => contract_class.try_into(),
            Cairo0ContractClass::Rpc(contract_class) => contract_class.try_into(),
        }
    }
}
