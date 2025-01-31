use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Serializer as JsonSerializer, Value};
use starknet_api::contract_class::EntryPointType;
use starknet_api::deprecated_contract_class::{
    ContractClass as DeprecatedContractClass, EntryPointV0,
};
use starknet_rs_core::types::Felt;
use starknet_types_core::hash::{Pedersen, StarkHash};

use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::felt::felt_from_prefixed_hex;
use crate::traits::HashProducer;
use crate::utils::StarknetFormatter;

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
        S: serde::Serializer,
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
/// are sorted alphabetically, which doesn't work for our case, because the contract artifact
/// contains keys under the "hints" property that are only numbers. So we use IndexMap to
/// preserve order of the keys, but its disadvantage is removing entries from the json object,
/// because it uses swap_remove method on IndexMap, which doesn't preserve order.
/// So we traverse the JSON object and remove all entries with key - attributes or
/// accessible_scopes if they are empty arrays.
fn compute_hinted_class_hash(contract_class: &Value) -> crate::error::DevnetResult<Felt> {
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
    let modified_abi_program_json = crate::utils::traverse_and_exclude_recursively(
        &abi_program_json,
        &|key, value| match value.as_array() {
            Some(array) if array.is_empty() => key == "attributes" || key == "accessible_scopes",
            _ => false,
        },
    );

    let mut buffer = Vec::with_capacity(128);
    let mut serializer = JsonSerializer::with_formatter(&mut buffer, StarknetFormatter);
    modified_abi_program_json.serialize(&mut serializer).map_err(JsonError::SerdeJsonError)?;

    Ok(starknet_rs_core::utils::starknet_keccak(&buffer))
}

// TODO perhaps rely on an imported util?
fn compute_cairo_0_contract_class_hash(json_class: &Value) -> crate::error::DevnetResult<Felt> {
    let mut hashes = vec![Felt::ZERO];

    let entry_points_by_type: HashMap<EntryPointType, Vec<EntryPointV0>> = serde_json::from_value(
        json_class
            .get("entry_points_by_type")
            .ok_or(JsonError::Custom { msg: "missing entry_points_by_type entry".to_string() })?
            .clone(),
    )
    .map_err(JsonError::SerdeJsonError)?;

    let entry_points_hash_by_type = |entry_point_type: EntryPointType| -> DevnetResult<Felt> {
        let felts: Vec<Felt> = entry_points_by_type
            .get(&entry_point_type)
            .ok_or(ConversionError::InvalidInternalStructure(
                "Missing entry point type".to_string(),
            ))?
            .iter()
            .flat_map(|entry_point| {
                let selector = entry_point.selector.0;
                let offset = Felt::from(entry_point.offset.0 as u128);

                vec![selector, offset]
            })
            .collect();

        Ok(Pedersen::hash_array(&felts))
    };

    hashes.push(entry_points_hash_by_type(EntryPointType::External)?);
    hashes.push(entry_points_hash_by_type(EntryPointType::L1Handler)?);
    hashes.push(entry_points_hash_by_type(EntryPointType::Constructor)?);

    let program_json = json_class
        .get("program")
        .ok_or(JsonError::Custom { msg: "missing program entry".to_string() })?;

    let builtins_encoded_as_felts = program_json
        .get("builtins")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .map(|el| {
            el.as_str()
                .map(|s| {
                    let hex_str = s
                        .as_bytes()
                        .iter()
                        .fold(String::from("0x"), |acc, &b| format!("{acc}{:02x}", b));
                    felt_from_prefixed_hex(&hex_str)
                })
                .ok_or(JsonError::Custom { msg: "expected string".into() })?
        })
        .collect::<DevnetResult<Vec<Felt>>>()?;

    hashes.push(Pedersen::hash_array(&builtins_encoded_as_felts));

    hashes.push(compute_hinted_class_hash(json_class)?);

    let program_data_felts = program_json
        .get("data")
        .unwrap_or(&serde_json::Value::Null)
        .as_array()
        .unwrap_or(&Vec::<serde_json::Value>::new())
        .clone()
        .into_iter()
        .map(|v| {
            felt_from_prefixed_hex(
                v.as_str().ok_or(JsonError::Custom { msg: "expected string".into() })?,
            )
        })
        .collect::<DevnetResult<Vec<Felt>>>()?;

    hashes.push(Pedersen::hash_array(&program_data_felts));

    Ok(Pedersen::hash_array(&hashes))
}

impl HashProducer for Cairo0ContractClass {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        match self {
            Cairo0ContractClass::Rpc(class) => {
                let class_json = serde_json::to_value(class).map_err(JsonError::SerdeJsonError)?;
                compute_cairo_0_contract_class_hash(&class_json)
            }
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
