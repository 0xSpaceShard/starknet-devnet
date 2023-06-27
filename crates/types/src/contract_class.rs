use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Serializer, Value};
use starknet_api::deprecated_contract_class::{EntryPoint, EntryPointType};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_in_rust::core::contract_address::compute_deprecated_class_hash;
use starknet_in_rust::core::errors::contract_address_errors::ContractAddressError;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::utils::calculate_sn_keccak;

use crate::error::{Error, JsonError};
use crate::felt::Felt;
use crate::traits::HashProducer;
use crate::{utils, DevnetResult};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ContractClassInner {
    StarknetInRust(StarknetInRustContractClass),
    JsonString(serde_json::Value),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContractClass(ContractClassInner);

impl ContractClass {
    pub fn from_json_str(json_str: &str) -> DevnetResult<Self> {
        let res: serde_json::Value =
            serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;
        if res.is_object() {
            Ok(Self(ContractClassInner::JsonString(res)))
        } else {
            Err(Error::JsonError(JsonError::Custom {
                msg: "expected JSON string to be an object".to_string(),
            }))
        }
    }
}

impl From<StarknetInRustContractClass> for ContractClass {
    fn from(value: StarknetInRustContractClass) -> Self {
        Self(ContractClassInner::StarknetInRust(value))
    }
}

impl TryFrom<ContractClass> for StarknetInRustContractClass {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value.0 {
            ContractClassInner::StarknetInRust(class) => Ok(class),
            ContractClassInner::JsonString(json_value) => {
                let sn_api: starknet_api::deprecated_contract_class::ContractClass =
                    serde_json::from_value(json_value).map_err(JsonError::SerdeJsonError)?;
                Ok(StarknetInRustContractClass::try_from(sn_api).unwrap())
            }
        }
    }
}

impl ContractClass {
    /// Computes the hinted class hash of the contract class.
    /// The parameter is a JSON object representing the contract class.
    /// Pythonic hinted class hash computation is based on a JSON artifact produced by the cairo-lang compiler.
    /// The JSON object contains his keys in alphabetical order. But when those keys are made of digits only, they are sorted in ascending order.
    /// For example keys "1", "10", "2" are sorted as "1", "2", "10" and keys "b", "a", "c" are sorted as "a", "b", "c".
    /// The resulting object is being serialized to a string and then hashed.
    /// In rust serde_json library when deserializing a JSON object, internally it uses a Map either HashMap or IndexMap. Depending on the feature enabled if [preserver_order] is not enabled HashMap will be used.
    /// In HashMap the keys order of insertion is not preserved and they are sorted alphabetically, which doesnt work for our case, because the contract artifact contains keys under the "hints" property that are only number_of_accounts
    /// So we use IndexMap, but is disadvantage is removing entries from the json object.
    /// So we traverse the JSON object and remove all entries with key attributes and accessible_scopes if they are empty arrays.
    /// And we are able to keep the JSON artifact order of keys insertion during deserialization.
    fn compute_hinted_class_hash(contract_class: &Value) -> crate::DevnetResult<StarkFelt> {
        let mut abi_json = json!({
            "abi": contract_class.get("abi").unwrap_or(&Value::Null),
            "program": contract_class.get("program").unwrap_or(&Value::Null)
        });

        let program_json = abi_json
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
        let res = crate::utils::traverse_and_exclude_recursively(&abi_json, &|key, value| {
            return (key == "attributes" || key == "accessible_scopes")
                && value.is_array()
                && value.as_array().expect("Not a valid JSON array").is_empty();
        });

        let mut writer = Vec::with_capacity(128);
        let mut serializer = Serializer::with_formatter(&mut writer, utils::StarknetFormatter);
        res.serialize(&mut serializer).map_err(JsonError::SerdeJsonError)?;

        let str_json = String::from_utf8(writer).map_err(|_| JsonError::Custom {
            msg: "Cannot convert from bytes to UTF-8 JSON string".to_string(),
        })?;

        Ok(StarkFelt::new(calculate_sn_keccak(str_json.as_bytes()))?)
    }
}

impl HashProducer for ContractClass {
    fn generate_hash(&self) -> crate::DevnetResult<crate::felt::Felt> {
        match &self.0 {
            ContractClassInner::StarknetInRust(class) => {
                let hash = compute_deprecated_class_hash(class)
                    .map_err(Error::StarknetInRustContractAddressError)?;
                Ok(Felt(hash.to_be_bytes()))
            }
            ContractClassInner::JsonString(json_class) => {
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
                        let non_prefixed_hex = json_str
                            .as_bytes()
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>();
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
                            str.as_str()
                                .ok_or(JsonError::Custom { msg: "expected string".to_string() })?,
                        )
                        .map_err(Error::StarknetApiError)
                    })
                    .collect::<DevnetResult<Vec<StarkFelt>>>()?;
                hashes.push(pedersen_hash_array(&program_data_felts));

                Ok(Felt::from(pedersen_hash_array(&hashes)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ContractClass;
    use crate::felt::Felt;
    use crate::traits::HashProducer;
    use crate::utils::test_utils::{CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};

    #[test]
    fn cairo_0_contract_class_hash_generated_successfully() {
        let json_str = std::fs::read_to_string(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let contract_class = ContractClass::from_json_str(&json_str).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();
        let expected_class_hash =
            Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap();
        assert_eq!(class_hash, expected_class_hash);
    }

    #[test]
    fn contract_class_from_json_str_doesnt_accept_string_different_from_json() {
        assert!(ContractClass::from_json_str(" not JSON string").is_err());
    }
}
