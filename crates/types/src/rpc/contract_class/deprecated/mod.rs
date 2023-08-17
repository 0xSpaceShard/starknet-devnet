use std::str::FromStr;

use starknet_in_rust::core::contract_address::compute_deprecated_class_hash;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_rs_core::types::CompressedLegacyContractClass;

use crate::contract_class::deprecated::rpc_contract_class::DeprecatedContractClass;
use crate::contract_class::Cairo0Json;
use crate::error::{Error, JsonError};
use crate::rpc::felt::Felt;
use crate::traits::HashProducer;
use crate::DevnetResult;

pub mod abi_entry;
pub mod json_contract_class;
pub mod rpc_contract_class;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cairo0ContractClass {
    // TODO: remove once starknet_api raised
    RawJson(Cairo0Json),
    SIR(StarknetInRustContractClass),
    Rpc(DeprecatedContractClass),
}

impl From<Cairo0Json> for Cairo0ContractClass {
    fn from(value: Cairo0Json) -> Self {
        Cairo0ContractClass::RawJson(value)
    }
}

impl From<StarknetInRustContractClass> for Cairo0ContractClass {
    fn from(value: StarknetInRustContractClass) -> Self {
        Cairo0ContractClass::SIR(value)
    }
}

impl From<DeprecatedContractClass> for Cairo0ContractClass {
    fn from(value: DeprecatedContractClass) -> Self {
        Cairo0ContractClass::Rpc(value)
    }
}

impl TryFrom<Cairo0ContractClass> for StarknetInRustContractClass {
    type Error = Error;
    fn try_from(value: Cairo0ContractClass) -> Result<Self, Self::Error> {
        match value {
            Cairo0ContractClass::RawJson(json_value) => {
                let starknet_in_rust_contract_class =
                    StarknetInRustContractClass::from_str(&json_value.to_string())
                        .map_err(|err| JsonError::Custom { msg: err.to_string() })?;
                Ok(starknet_in_rust_contract_class)
            }
            Cairo0ContractClass::SIR(contract) => Ok(contract),
            Cairo0ContractClass::Rpc(contract) => contract.try_into(),
        }
    }
}

impl HashProducer for Cairo0ContractClass {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        match self {
            Cairo0ContractClass::RawJson(contract_json) => Ok(contract_json.generate_hash()?),
            Cairo0ContractClass::SIR(contract) => {
                Ok(compute_deprecated_class_hash(contract)?.into())
            }
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
            Cairo0ContractClass::SIR(_) => {
                Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat))
            }
        }
    }
}
