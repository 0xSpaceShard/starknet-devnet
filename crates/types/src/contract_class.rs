use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Serializer as JsonSerializer, Value};
use starknet_api::deprecated_contract_class::{EntryPoint, EntryPointType};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_in_rust::core::contract_address::{
    compute_deprecated_class_hash, compute_sierra_class_hash,
};
use starknet_in_rust::core::errors::contract_address_errors::ContractAddressError;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::utils::calculate_sn_keccak;
use starknet_in_rust::SierraContractClass;

use crate::abi_entry::{AbiEntry, AbiEntryType};
use crate::error::{Error, JsonError};
use crate::felt::Felt;
use crate::serde_helpers::base_64_gzipped_json_string::deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order;
use crate::traits::HashProducer;
use crate::{utils, DevnetResult};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractClassAbiEntryWithType {
    #[serde(flatten)]
    pub entry: AbiEntry,
    pub r#type: AbiEntryType,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeprecatedContractClass {
    pub abi: Vec<ContractClassAbiEntryWithType>,
    /// A base64 encoding of the gzip-compressed JSON representation of program.
    #[serde(
        deserialize_with = "deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order"
    )]
    pub program: Value,
    /// The selector of each entry point is a unique identifier in the program.
    pub entry_points_by_type: HashMap<EntryPointType, Vec<EntryPoint>>,
}

pub type Cairo0Json = Value;

impl HashProducer for Cairo0Json {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(ContractClass::compute_cairo_0_contract_class_hash(&self)?.into())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cairo0ContractClass {
    RawJson(Cairo0Json),
    SIR(StarknetInRustContractClass),
    Rpc(DeprecatedContractClass),
}

impl Cairo0ContractClass {
    pub fn raw_json_from_json_str(json_str: &str) -> DevnetResult<Cairo0Json> {
        let res: Cairo0Json = serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;

        if res.is_object() {
            Ok(res)
        } else {
            Err(Error::JsonError(JsonError::Custom {
                msg: "expected JSON string to be an object".to_string(),
            }))
        }
    }

    pub fn raw_json_from_path(path: &str) -> DevnetResult<Cairo0Json> {
        let contract_class_str = fs::read_to_string(path)?;
        Ok(Cairo0ContractClass::raw_json_from_json_str(&contract_class_str)?)
    }

    pub fn rpc_from_json_str(json_str: &str) -> DevnetResult<DeprecatedContractClass> {
        let res: DeprecatedContractClass =
            serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;

        Ok(res)
    }

    pub fn rpc_from_path(path: &str) -> DevnetResult<DeprecatedContractClass> {
        let contract_class_str = fs::read_to_string(path)?;
        Ok(Cairo0ContractClass::rpc_from_json_str(&contract_class_str)?)
    }
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

impl TryFrom<DeprecatedContractClass> for Cairo0Json {
    type Error = Error;
    fn try_from(value: DeprecatedContractClass) -> Result<Self, Self::Error> {
        let abi_json = serde_json::to_value(value.abi).map_err(JsonError::SerdeJsonError)?;
        let entry_points_json =
            serde_json::to_value(value.entry_points_by_type).map_err(JsonError::SerdeJsonError)?;

        let json_value = json!({
            "program": value.program,
            "abi": abi_json,
            "entry_points_by_type": entry_points_json,
        });

        Ok(json_value)
    }
}

impl TryFrom<DeprecatedContractClass> for StarknetInRustContractClass {
    type Error = Error;
    fn try_from(value: DeprecatedContractClass) -> Result<Self, Self::Error> {
        let json_value: Value = value.try_into()?;
        let starknet_in_rust_contract_class =
            StarknetInRustContractClass::from_str(&json_value.to_string())
                .map_err(|err| JsonError::Custom { msg: err.to_string() })?;

        Ok(starknet_in_rust_contract_class)
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
            ContractClass::Cairo0(cairo_0) => match cairo_0 {
                Cairo0ContractClass::RawJson(contract) => Ok(contract),
                _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
            },
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl ContractClass {
    /// Computes the hinted class hash of the contract class.
    /// The parameter is a JSON object representing the contract class.
    /// Pythonic hinted class hash computation is based on a JSON artifact produced by the
    /// cairo-lang compiler. The JSON object contains his keys in alphabetical order. But when
    /// those keys are made of digits only, they are sorted in ascending order. For example keys
    /// "1", "10", "2" are sorted as "1", "2", "10" and keys "b", "a", "c" are sorted as "a", "b",
    /// "c". The resulting object is being serialized to a string and then hashed.
    /// In rust serde_json library when deserializing a JSON object, internally it uses a Map either
    /// HashMap or IndexMap. Depending on the feature enabled if [preserver_order] is not enabled
    /// HashMap will be used. In HashMap the keys order of insertion is not preserved and they
    /// are sorted alphabetically, which doesnt work for our case, because the contract artifact
    /// contains keys under the "hints" property that are only numbers. So we use IndexMap to
    /// preserve order of the keys, but its disadvantage is removing entries from the json object,
    /// because it uses swap_remove method on IndexMap, which doesnt preserve order.
    /// So we traverse the JSON object and remove all entries with key - attributes or
    /// accessible_scopes if they are empty arrays.
    fn compute_hinted_class_hash(contract_class: &Value) -> crate::DevnetResult<StarkFelt> {
        let mut abi_program_json = json!({
            "abi": contract_class.get("abi").unwrap_or(&Value::Null),
            "program": contract_class.get("program").unwrap_or(&Value::Null)
        });
        let program_json = abi_program_json
            .get_mut("program")
            .ok_or(JsonError::Custom { msg: "missing program entry".to_string() })?;

        let debug_info_json = program_json.get_mut("debug_info");
        if debug_info_json.is_some() {
            program_json
                .as_object_mut()
                .ok_or(JsonError::Custom { msg: "expected object".to_string() })?
                .insert("debug_info".to_string(), serde_json::Value::Null);
        }

        // Traverse the JSON and remove all entries with key attributes and accessible_scopes
        // if they are empty arrays.
        let modified_abi_program_json =
            crate::utils::traverse_and_exclude_recursively(&abi_program_json, &|key, value| {
                return (key == "attributes" || key == "accessible_scopes")
                    && value.is_array()
                    && value.as_array().expect("Not a valid JSON array").is_empty();
            });

        let mut buffer = Vec::with_capacity(128);
        let mut serializer = JsonSerializer::with_formatter(&mut buffer, utils::StarknetFormatter);
        modified_abi_program_json.serialize(&mut serializer).map_err(JsonError::SerdeJsonError)?;

        Ok(StarkFelt::new(calculate_sn_keccak(&buffer))?)
    }

    fn compute_cairo_0_contract_class_hash(json_class: &Value) -> crate::DevnetResult<Felt> {
        let mut hashes = Vec::<StarkFelt>::new();
        hashes.push(StarkFelt::from(0u128));

        let entry_points_by_type: HashMap<EntryPointType, Vec<EntryPoint>> =
            serde_json::from_value(
                json_class
                    .get("entry_points_by_type")
                    .ok_or(JsonError::Custom {
                        msg: "missing entry_points_by_type entry".to_string(),
                    })?
                    .clone(),
            )
            .unwrap();

        let entry_points_hash_by_type =
            |entry_point_type: EntryPointType| -> DevnetResult<StarkFelt> {
                let felts: Vec<StarkFelt> = entry_points_by_type
                    .get(&entry_point_type)
                    .ok_or(ContractAddressError::NoneExistingEntryPointType)?
                    .iter()
                    .flat_map(|entry_point| {
                        let selector = entry_point.selector.0;
                        let offset = StarkFelt::from(entry_point.offset.0 as u128);

                        vec![selector, offset]
                    })
                    .collect();

                Ok(pedersen_hash_array(&felts))
            };

        hashes.push(entry_points_hash_by_type(EntryPointType::External)?);
        hashes.push(entry_points_hash_by_type(EntryPointType::L1Handler)?);
        hashes.push(entry_points_hash_by_type(EntryPointType::Constructor)?);

        let program_json = json_class
            .get("program")
            .ok_or(JsonError::Custom { msg: "missing program entry".to_string() })?;
        let builtins_encoded_as_felts = program_json
            .get("builtins")
            .unwrap_or(&serde_json::Value::Null)
            .as_array()
            .unwrap_or(&Vec::<serde_json::Value>::new())
            .iter()
            .map(|el| {
                let json_str = el.as_str().unwrap();
                let non_prefixed_hex =
                    json_str.as_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>();
                let prefixed_hex = format!("0x{}", non_prefixed_hex);
                prefixed_hex
            })
            .collect::<Vec<String>>()
            .into_iter()
            .map(|el| StarkFelt::try_from(el.as_str()).map_err(Error::StarknetApiError))
            .collect::<DevnetResult<Vec<StarkFelt>>>()?;

        hashes.push(pedersen_hash_array(&builtins_encoded_as_felts));

        hashes.push(ContractClass::compute_hinted_class_hash(json_class)?);

        let program_data_felts = program_json
            .get("data")
            .unwrap_or(&serde_json::Value::Null)
            .as_array()
            .unwrap_or(&Vec::<serde_json::Value>::new())
            .clone()
            .into_iter()
            .map(|str| {
                StarkFelt::try_from(
                    str.as_str().ok_or(JsonError::Custom { msg: "expected string".to_string() })?,
                )
                .map_err(Error::StarknetApiError)
            })
            .collect::<DevnetResult<Vec<StarkFelt>>>()?;
        hashes.push(pedersen_hash_array(&program_data_felts));

        Ok(Felt::from(pedersen_hash_array(&hashes)))
    }
}

impl HashProducer for DeprecatedContractClass {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        let json_value: Cairo0Json = self.clone().try_into()?;
        json_value.generate_hash()
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

#[cfg(test)]
mod tests {
    use crate::contract_class::Cairo0ContractClass;
    use core::panic;

    use super::ContractClass;
    use crate::felt::Felt;
    use crate::traits::HashProducer;
    use crate::utils::test_utils::{CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};

    #[test]
    #[ignore]
    fn cairo_1_contract_class_hash_generated_successfully() {
        panic!("Add check with expected class hash generated from sierra");
    }

    #[test]
    fn cairo_0_rpc_successfully() {
        let json_str = std::fs::read_to_string("/Users/edwin/Documents/work/ShardLabs/starknet-devnet-rs/crates/starknet/test_artifacts/cairo_0_rpc.json").unwrap();
        let contract_class = Cairo0ContractClass::rpc_from_json_str(&json_str).unwrap();
        let hash = contract_class.generate_hash().unwrap();
    }

    #[test]
    fn cairo_0_contract_class_hash_generated_successfully() {
        let json_str = std::fs::read_to_string(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let contract_class = Cairo0ContractClass::raw_json_from_json_str(&json_str).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();
        let expected_class_hash =
            Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap();
        assert_eq!(class_hash, expected_class_hash);
    }

    #[test]
    fn contract_class_cairo_0_from_json_str_doesnt_accept_string_different_from_json() {
        assert!(Cairo0ContractClass::raw_json_from_json_str(" not JSON string").is_err());
    }
}
