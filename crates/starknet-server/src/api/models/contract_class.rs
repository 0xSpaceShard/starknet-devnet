use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::abi_entry::{AbiEntry, AbiEntryType};
use super::FeltHex;
use crate::api::serde_helpers::base_64_gzipped_json_string::deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContractClass {
    Cairo0(DeprecatedContractClass),
    Sierra(starknet_in_rust::SierraContractClass),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct SierraContractClass {
    pub sierra_program: Vec<FeltHex>,
    pub contract_class_version: String,
    pub entry_points_by_type: HashMap<
        starknet_types::starknet_api::state::EntryPointType,
        Vec<starknet_types::starknet_api::state::EntryPoint>,
    >,
    pub abi: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeprecatedContractClass {
    pub abi: Vec<ContractClassAbiEntryWithType>,
    /// A base64 encoding of the gzip-compressed JSON representation of program.
    #[serde(
        deserialize_with = "deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order"
    )]
    pub program: serde_json::Value,
    /// The selector of each entry point is a unique identifier in the program.
    pub entry_points_by_type: HashMap<
        starknet_types::starknet_api::deprecated_contract_class::EntryPointType,
        Vec<starknet_types::starknet_api::deprecated_contract_class::EntryPoint>,
    >,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractClassAbiEntryWithType {
    #[serde(flatten)]
    pub entry: AbiEntry,
    pub r#type: AbiEntryType,
}

#[cfg(test)]
mod tests {
    use starknet_types::felt::Felt;

    use crate::api::models::abi_entry::FunctionAbiEntry;

    #[test]
    fn deserialize_contract_class_abi_entry_with_type() {
        let json_str = r#"{
            "inputs": [],
            "name": "getPublicKey",
            "outputs": [
                {
                    "name": "publicKey",
                    "type": "felt"
                }
            ],
            "stateMutability": "view",
            "type": "function"
        }"#;

        let obj = serde_json::from_str::<super::ContractClassAbiEntryWithType>(json_str).unwrap();
        assert_eq!(obj.r#type, super::AbiEntryType::Function);
        assert_eq!(
            obj.entry,
            super::AbiEntry::Function(FunctionAbiEntry {
                name: "getPublicKey".to_string(),
                inputs: vec![],
                outputs: vec![
                    starknet_types::starknet_api::deprecated_contract_class::TypedParameter {
                        name: "publicKey".to_string(),
                        r#type: "felt".to_string(),
                    }
                ],
                state_mutability: Some("view".to_string()),
            })
        );

        let json_str = r#"{
            "inputs": [
                {
                    "name": "newPublicKey",
                    "type": "felt"
                }
            ],
            "name": "setPublicKey",
            "outputs": [],
            "type": "function"
        }"#;

        let obj = serde_json::from_str::<super::ContractClassAbiEntryWithType>(json_str).unwrap();
        assert_eq!(obj.r#type, super::AbiEntryType::Function);
        assert_eq!(
            obj.entry,
            super::AbiEntry::Function(FunctionAbiEntry {
                name: "setPublicKey".to_string(),
                inputs: vec![
                    starknet_types::starknet_api::deprecated_contract_class::TypedParameter {
                        name: "newPublicKey".to_string(),
                        r#type: "felt".to_string(),
                    }
                ],
                outputs: vec![],
                state_mutability: None,
            })
        );

        let json_str = r#"{
            "inputs": [
                {
                    "name": "publicKey",
                    "type": "felt"
                }
            ],
            "name": "constructor",
            "outputs": [],
            "type": "constructor"
        }"#;

        let obj = serde_json::from_str::<super::ContractClassAbiEntryWithType>(json_str).unwrap();
        assert_eq!(obj.r#type, super::AbiEntryType::Constructor);
        assert_eq!(
            obj.entry,
            super::AbiEntry::Function(FunctionAbiEntry {
                name: "constructor".to_string(),
                inputs: vec![
                    starknet_types::starknet_api::deprecated_contract_class::TypedParameter {
                        name: "publicKey".to_string(),
                        r#type: "felt".to_string(),
                    }
                ],
                outputs: vec![],
                state_mutability: None,
            })
        );
    }
    #[test]
    fn deserialize_deprecated_contract_class() {
        let json_str = r#"{
            "abi": [
                {
                    "inputs": [],
                    "name": "getPublicKey",
                    "outputs": [
                        {
                            "name": "publicKey",
                            "type": "felt"
                        }
                    ],
                    "stateMutability": "view",
                    "type": "function"
                },
                {
                    "inputs": [
                        {
                            "name": "newPublicKey",
                            "type": "felt"
                        }
                    ],
                    "name": "setPublicKey",
                    "outputs": [],
                    "type": "function"
                },
                {
                    "inputs": [
                        {
                            "name": "publicKey",
                            "type": "felt"
                        }
                    ],
                    "name": "constructor",
                    "outputs": [],
                    "type": "constructor"
                }
            ],
            "program": "",
            "entry_points_by_type": {
                "EXTERNAL": [
                    {
                        "selector": "0xAAE3B5E8",
                        "offset": "0x1"
                    },
                    {
                        "selector": "0xAAE3B5E9",
                        "offset": "0x2"
                    }
                ]
            }
        }"#;

        let obj = serde_json::from_str::<super::DeprecatedContractClass>(json_str).unwrap();
        assert_eq!(obj.abi.len(), 3);
        assert_eq!(obj.entry_points_by_type.len(), 1);
        assert_eq!(obj.entry_points_by_type.get(&starknet_types::starknet_api::deprecated_contract_class::EntryPointType::External).unwrap().len(), 2);
    }

    #[test]
    fn deserialize_sierra_contract_class() {
        let json_str = r#"{
            "sierra_program": ["0xAA", "0xBB"],
            "contract_class_version": "1.0",
            "entry_points_by_type": {
                "EXTERNAL": [
                    {
                        "selector": "0xAAE3B5E8",
                        "function_idx": 1
                    },
                    {
                        "selector": "0xAAE3B5E9",
                        "function_idx": 2
                    }
                ]
            },
            "abi": "H4sIAAAAAAAA/8tIzcnJVyjPL8pJUQQAlQYXAAAA"
        }"#;
        let obj = serde_json::from_str::<super::SierraContractClass>(json_str).unwrap();
        assert_eq!(obj.sierra_program.len(), 2);
        assert_eq!(obj.contract_class_version, "1.0".to_string());
        assert_eq!(obj.entry_points_by_type.len(), 1);
        assert_eq!(
            obj.entry_points_by_type
                .get(&starknet_types::starknet_api::state::EntryPointType::External)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(obj.abi, "H4sIAAAAAAAA/8tIzcnJVyjPL8pJUQQAlQYXAAAA".to_string());
        assert_eq!(
            obj.entry_points_by_type
                .get(&starknet_types::starknet_api::state::EntryPointType::External)
                .unwrap()[0]
                .selector
                .0,
            starknet_types::starknet_api::hash::StarkFelt::from(
                Felt::from_prefixed_hex_str("0xAAE3B5E8").unwrap()
            )
        );
    }
}
