use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::default::Default;
use std::fs;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Serializer as JsonSerializer, Value};
use starknet_api::deprecated_contract_class::{EntryPoint, EntryPointType};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_in_rust::core::errors::contract_address_errors::ContractAddressError;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::utils::calculate_sn_keccak;
use starknet_in_rust::SierraContractClass;
use starknet_rs_core::types::{
    ContractClass as CodegenContractClass, FlattenedSierraClass as CodegenSierraContracrClass,
};

use crate::error::JsonError::SerdeJsonError;
use crate::error::{ConversionError, Error, JsonError};
use crate::felt::Felt;
use crate::traits::HashProducer;
use crate::{utils, DevnetResult};
use base64::Engine;
use core::fmt::{Debug, Display};
use flate2::write::GzEncoder;
use flate2::Compression;
use starknet_in_rust::core::contract_address::compute_sierra_class_hash;
use starknet_rs_core::serde::byte_array::base64 as base64Sir;
use starknet_rs_core::types::contract::legacy::LegacyProgram;
use starknet_rs_core::types::{CompressedLegacyContractClass, LegacyEntryPointsByType};
use std::fmt::Formatter;

pub mod deprecated;
pub use deprecated::json_contract_class::Cairo0Json;
pub use deprecated::rpc_contract_class::DeprecatedContractClass;
pub use deprecated::Cairo0ContractClass;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ContractClass {
    Cairo0(Cairo0ContractClass),
    Cairo1(SierraContractClass),
}

impl Serialize for ContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ContractClass::Cairo0(cairo0) => match cairo0 {
                Cairo0ContractClass::RawJson(contract_json) => contract_json.serialize(serializer),
                Cairo0ContractClass::SIR(_) => Err(serde::ser::Error::custom(
                    "Serialization of starknet 0 contract is unavailable",
                )),
                Cairo0ContractClass::Rpc(contract) => contract.serialize(serializer),
            },
            ContractClass::Cairo1(contract) => contract.serialize(serializer),
        }
    }
}

impl ContractClass {
    pub fn cairo_1_from_sierra_json_str(json_str: &str) -> DevnetResult<SierraContractClass> {
        let sierra_contract_class: SierraContractClass =
            serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;

        Ok(sierra_contract_class)
    }
}

impl From<Cairo0ContractClass> for ContractClass {
    fn from(value: Cairo0ContractClass) -> Self {
        ContractClass::Cairo0(value)
    }
}

impl From<StarknetInRustContractClass> for ContractClass {
    fn from(value: StarknetInRustContractClass) -> Self {
        ContractClass::Cairo0(value.into())
    }
}

impl From<DeprecatedContractClass> for ContractClass {
    fn from(value: DeprecatedContractClass) -> Self {
        ContractClass::Cairo0(value.into())
    }
}

impl From<Cairo0Json> for ContractClass {
    fn from(value: Cairo0Json) -> Self {
        ContractClass::Cairo0(value.into())
    }
}

impl From<SierraContractClass> for ContractClass {
    fn from(value: SierraContractClass) -> Self {
        ContractClass::Cairo1(value)
    }
}

impl TryFrom<ContractClass> for SierraContractClass {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo1(sierra) => Ok(sierra),
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl TryFrom<ContractClass> for Cairo0ContractClass {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo0(cairo_0) => Ok(cairo_0),
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl TryFrom<ContractClass> for Cairo0Json {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo0(Cairo0ContractClass::RawJson(contract)) => Ok(contract),
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl HashProducer for ContractClass {
    fn generate_hash(&self) -> crate::DevnetResult<crate::felt::Felt> {
        match self {
            ContractClass::Cairo0(contract) => Ok(contract.generate_hash()?),
            ContractClass::Cairo1(sierra) => {
                let sierra_felt252_hash = compute_sierra_class_hash(sierra)?;
                Ok(Felt::from(sierra_felt252_hash))
            }
        }
    }
}

fn convert_sierra_to_codegen(
    contract_class: &SierraContractClass,
) -> DevnetResult<CodegenSierraContracrClass> {
    // TODO: improve
    let value: Value =
        serde_json::to_value(contract_class.clone()).map_err(JsonError::SerdeJsonError)?;

    Ok(serde_json::from_value(value).map_err(JsonError::SerdeJsonError)?)
}

impl TryInto<CodegenContractClass> for ContractClass {
    type Error = Error;
    fn try_into(self) -> Result<CodegenContractClass, Self::Error> {
        match self {
            ContractClass::Cairo0(contract_class) => {
                Ok(CodegenContractClass::Legacy(contract_class.try_into()?))
            }
            ContractClass::Cairo1(contract_class) => {
                Ok(CodegenContractClass::Sierra(convert_sierra_to_codegen(&contract_class)?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::contract_class::{Cairo0ContractClass, Cairo0Json, DeprecatedContractClass};
    use core::panic;
    use starknet_rs_core::types::CompressedLegacyContractClass;

    use crate::felt::Felt;
    use crate::traits::{HashProducer, ToHexString};
    use crate::utils::test_utils::{CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};

    #[test]
    #[ignore]
    fn cairo_1_contract_class_hash_generated_successfully() {
        panic!("Add check with expected class hash generated from sierra");
    }

    #[test]
    fn cairo_0_contract_class_hash_generated_successfully() {
        let json_str = std::fs::read_to_string(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let contract_class = Cairo0Json::raw_json_from_json_str(&json_str).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();
        let expected_class_hash =
            Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap();
        assert_eq!(class_hash, expected_class_hash);
    }

    #[test]
    fn contract_class_cairo_0_from_json_str_doesnt_accept_string_different_from_json() {
        assert!(Cairo0Json::raw_json_from_json_str(" not JSON string").is_err());
    }
}
