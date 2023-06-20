use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::FeltHex;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContractClass {
    Cairo0(DeprecatedContractClass),
    Sierra(SierraContractClass),
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
    pub program: String,
    /// The selector of each entry point is a unique identifier in the program.
    pub entry_points_by_type: HashMap<
        starknet_types::starknet_api::deprecated_contract_class::EntryPointType,
        Vec<starknet_types::starknet_api::deprecated_contract_class::EntryPoint>,
    >,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractClassAbiEntryWithType {
    pub r#type: ContractClassAbiEntryType,
    #[serde(flatten)]
    pub entry: ContractClassAbiEntry,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
pub enum ContractClassAbiEntryType {
    #[serde(rename(deserialize = "constructor", serialize = "constructor"))]
    Constructor,
    #[serde(rename(deserialize = "event", serialize = "event"))]
    Event,
    #[serde(rename(deserialize = "function", serialize = "function"))]
    #[default]
    Function,
    #[serde(rename(deserialize = "l1_handler", serialize = "l1_handler"))]
    L1Handler,
    #[serde(rename(deserialize = "struct", serialize = "struct"))]
    Struct,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum ContractClassAbiEntry {
    /// An event abi entry.
    Event(starknet_types::starknet_api::deprecated_contract_class::EventAbiEntry),
    /// A function abi entry.
    Function(starknet_types::starknet_api::deprecated_contract_class::FunctionAbiEntry),
    /// A struct abi entry.
    Struct(starknet_types::starknet_api::deprecated_contract_class::StructAbiEntry),
}
