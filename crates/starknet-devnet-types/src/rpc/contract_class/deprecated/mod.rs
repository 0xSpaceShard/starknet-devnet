//use crate::contract_class::deprecated::rpc_contract_class::DeprecatedContractClass;
use crate::contract_class::Cairo0Json;
use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use crate::traits::HashProducer;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_api::deprecated_contract_class::ContractClass;
use starknet_rs_core::types::CompressedLegacyContractClass;

pub mod abi_entry;
pub mod json_contract_class;
pub mod rpc_contract_class;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StarknetApiContractClass {
    // TODO: remove once starknet_api raised
    RawJson(Cairo0Json),
    Rpc(ContractClass),
}

impl Serialize for StarknetApiContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StarknetApiContractClass::RawJson(contract_json) => contract_json.serialize(serializer),
            StarknetApiContractClass::Rpc(contract) => contract.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StarknetApiContractClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(StarknetApiContractClass::Rpc(ContractClass::deserialize(deserializer)?))
    }
}

impl From<Cairo0Json> for StarknetApiContractClass {
    fn from(value: Cairo0Json) -> Self {
        StarknetApiContractClass::RawJson(value)
    }
}

impl From<ContractClass> for StarknetApiContractClass {
    fn from(value: ContractClass) -> Self {
        StarknetApiContractClass::Rpc(value)
    }
}

impl HashProducer for StarknetApiContractClass {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        match self {
            StarknetApiContractClass::RawJson(contract_json) => Ok(contract_json.generate_hash()?),
            StarknetApiContractClass::Rpc(contract) => Ok(contract.generate_hash()?),
        }
    }
}

impl TryInto<CompressedLegacyContractClass> for StarknetApiContractClass {
    type Error = Error;
    fn try_into(self) -> Result<CompressedLegacyContractClass, Self::Error> {
        match self {
            StarknetApiContractClass::Rpc(contract_class) => contract_class.try_into(),
            StarknetApiContractClass::RawJson(contract_class) => contract_class.try_into(),
        }
    }
}

impl TryFrom<StarknetApiContractClass> for blockifier::execution::contract_class::ContractClassV0 {
    type Error = Error;
    fn try_from(value: StarknetApiContractClass) -> Result<Self, Self::Error> {
        match value {
            StarknetApiContractClass::RawJson(contract_class) => contract_class.try_into(),
            StarknetApiContractClass::Rpc(contract_class) => contract_class.try_into(),
        }
    }
}
