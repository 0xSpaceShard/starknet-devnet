use core::fmt::Debug;
use std::cmp::{Eq, PartialEq};

use serde::{Serialize, Serializer};
use starknet_in_rust::core::contract_address::compute_sierra_class_hash;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::SierraContractClass;
use starknet_rs_core::types::{
    ContractClass as CodegenContractClass, FlattenedSierraClass as CodegenSierraContracrClass,
};
use starknet_rs_ff::FieldElement;

use crate::error::{DevnetResult, Error, JsonError};
use crate::felt::Felt;
use crate::traits::HashProducer;

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
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
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
    let abi = serde_json::to_string(&contract_class.abi).map_err(JsonError::SerdeJsonError)?;
    let sierra_program = contract_class
        .sierra_program
        .iter()
        .map(|bigint| {
            FieldElement::from_byte_slice_be(&bigint.value.to_bytes_be())
                .map_err(Error::StarknetFfConversionError)
        })
        .collect::<DevnetResult<Vec<FieldElement>>>()?;

    let entry_points_by_type_value =
        serde_json::to_value(contract_class.entry_points_by_type.clone())
            .map_err(JsonError::SerdeJsonError)?;
    let entry_points_by_type =
        serde_json::from_value(entry_points_by_type_value).map_err(JsonError::SerdeJsonError)?;

    Ok(CodegenSierraContracrClass {
        sierra_program,
        contract_class_version: contract_class.contract_class_version.clone(),
        entry_points_by_type,
        abi,
    })
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
    use serde_json::Deserializer;
    use starknet_in_rust::SierraContractClass;

    use crate::contract_class::{convert_sierra_to_codegen, Cairo0Json, ContractClass};
    use crate::felt::Felt;
    use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
    use crate::traits::HashProducer;
    use crate::utils::test_utils::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH, CAIRO_1_CONTRACT_PATH,
        CAIRO_1_CONTRACT_SIERRA_HASH,
    };

    #[test]
    fn cairo_1_contract_class_hash_generated_successfully() {
        let cairo_1_contract_sierra = ContractClass::Cairo1(
            ContractClass::cairo_1_from_sierra_json_str(
                &std::fs::read_to_string(CAIRO_1_CONTRACT_PATH).unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            Felt::from_prefixed_hex_str(CAIRO_1_CONTRACT_SIERRA_HASH).unwrap(),
            cairo_1_contract_sierra.generate_hash().unwrap()
        );
    }

    #[test]
    fn cairo_1_to_codegen() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/sierra_contract_class_with_abi_as_string.json"
        );
        let contract_str = std::fs::read_to_string(path).unwrap();
        let mut deserializer = Deserializer::from_str(&contract_str);
        let contract_class: SierraContractClass =
            deserialize_to_sierra_contract_class(&mut deserializer).unwrap();

        convert_sierra_to_codegen(&contract_class).unwrap();
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
