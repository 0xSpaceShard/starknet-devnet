use core::fmt::{Debug, Display, Formatter};
use std::collections::HashMap;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use serde_json::{Serializer as JsonSerializer, Value, json};
use starknet_api::contract_class::EntryPointType;
use starknet_api::deprecated_contract_class::EntryPointV0;
use starknet_rs_core::types::{CompressedLegacyContractClass, Felt};
use starknet_types_core::hash::{Pedersen, StarkHash};

use crate::contract_class::deprecated::rpc_contract_class::DeprecatedContractClass;
use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::felt::felt_from_prefixed_hex;
use crate::traits::HashProducer;
use crate::utils::StarknetFormatter;

#[derive(Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Cairo0Json {
    pub inner: Value,
}

impl Cairo0Json {
    pub fn raw_json_from_json_str(json_str: &str) -> DevnetResult<Cairo0Json> {
        let value: Value = serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;

        if value.is_object() {
            Ok(Cairo0Json { inner: value })
        } else {
            Err(Error::JsonError(JsonError::Custom {
                msg: "expected JSON string to be an object".to_string(),
            }))
        }
    }

    pub fn raw_json_from_path(path: &str) -> DevnetResult<Cairo0Json> {
        let contract_class_str = std::fs::read_to_string(path)?;
        Cairo0Json::raw_json_from_json_str(&contract_class_str)
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
        let modified_abi_program_json =
            crate::utils::traverse_and_exclude_recursively(&abi_program_json, &|key, value| {
                match value.as_array() {
                    Some(array) if array.is_empty() => {
                        key == "attributes" || key == "accessible_scopes"
                    }
                    _ => false,
                }
            });

        let mut buffer = Vec::with_capacity(128);
        let mut serializer = JsonSerializer::with_formatter(&mut buffer, StarknetFormatter);
        modified_abi_program_json.serialize(&mut serializer).map_err(JsonError::SerdeJsonError)?;

        Ok(starknet_rs_core::utils::starknet_keccak(&buffer))
    }

    fn compute_cairo_0_contract_class_hash(json_class: &Value) -> crate::error::DevnetResult<Felt> {
        let mut hashes = vec![Felt::ZERO];

        let entry_points_by_type: HashMap<EntryPointType, Vec<EntryPointV0>> =
            serde_json::from_value(
                json_class
                    .get("entry_points_by_type")
                    .ok_or(JsonError::Custom {
                        msg: "missing entry_points_by_type entry".to_string(),
                    })?
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

        hashes.push(Cairo0Json::compute_hinted_class_hash(json_class)?);

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
}

impl Display for Cairo0Json {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl TryInto<CompressedLegacyContractClass> for Cairo0Json {
    type Error = Error;
    fn try_into(self) -> Result<CompressedLegacyContractClass, Self::Error> {
        let value = self.inner;

        let abi = value.get("abi");
        let entry_points_by_type = value
            .get("entry_points_by_type")
            .ok_or(JsonError::Custom { msg: "expected object".to_string() })?;
        let program =
            value.get("program").ok_or(JsonError::Custom { msg: "expected object".to_string() })?;
        let compressed_program = json_into_raw_program(program)?;

        match abi {
            Some(abi_json) => Ok(CompressedLegacyContractClass {
                program: compressed_program,
                entry_points_by_type: serde_json::from_value(entry_points_by_type.clone())
                    .map_err(JsonError::SerdeJsonError)?,
                abi: serde_json::from_value(abi_json.clone()).map_err(JsonError::SerdeJsonError)?,
            }),
            None => Ok(CompressedLegacyContractClass {
                program: compressed_program,
                entry_points_by_type: serde_json::from_value(entry_points_by_type.clone())
                    .map_err(JsonError::SerdeJsonError)?,
                abi: None,
            }),
        }
    }
}

impl HashProducer for Cairo0Json {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Cairo0Json::compute_cairo_0_contract_class_hash(&self.inner)
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

        Ok(Cairo0Json { inner: json_value })
    }
}

impl TryFrom<Cairo0Json> for starknet_api::deprecated_contract_class::ContractClass {
    type Error = Error;

    fn try_from(value: Cairo0Json) -> Result<Self, Self::Error> {
        serde_json::from_value(value.inner)
            .map_err(|err| Error::JsonError(JsonError::SerdeJsonError(err)))
    }
}

pub fn json_into_raw_program(json_data: &Value) -> DevnetResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let encoder = GzEncoder::new(&mut buffer, Compression::best());
    serde_json::to_writer(encoder, &json_data).map_err(JsonError::SerdeJsonError)?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::CompressedLegacyContractClass;

    use crate::contract_class::deprecated::Cairo0Json;
    use crate::utils::test_utils::CAIRO_0_ACCOUNT_CONTRACT_PATH;

    #[test]
    fn test_unzipped_to_codegen_conversion() {
        let class = Cairo0Json::raw_json_from_path(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let _: CompressedLegacyContractClass = class.try_into().unwrap();
    }
}
