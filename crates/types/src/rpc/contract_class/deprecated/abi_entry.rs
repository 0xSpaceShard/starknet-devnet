use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
pub enum AbiEntryType {
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
#[serde(untagged)]
pub enum AbiEntry {
    /// An event abi entry.
    Event(EventAbiEntry),
    /// A function abi entry.
    Function(FunctionAbiEntry),
    /// A struct abi entry.
    Struct(StructAbiEntry),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventAbiEntry {
    pub data: Vec<starknet_api::deprecated_contract_class::TypedParameter>,
    pub keys: Vec<starknet_api::deprecated_contract_class::TypedParameter>,
    pub name: String,
}

/// A struct abi entry.
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StructAbiEntry {
    pub members: Vec<StructMember>,
    pub name: String,
    pub size: usize,
}

/// A struct member for [StructAbiEntry](`crate::deprecated_contract_class::StructAbiEntry`).
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StructMember {
    pub name: String,
    pub offset: usize,
    pub r#type: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct FunctionAbiEntry {
    pub inputs: Vec<starknet_api::deprecated_contract_class::TypedParameter>,
    pub name: String,
    pub outputs: Vec<starknet_api::deprecated_contract_class::TypedParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stateMutability")]
    pub state_mutability: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::FunctionAbiEntry;

    #[test]
    fn deserialize_function_abi_entry() {
        let json_str = r#"{
                "name": "foo",
                "inputs": [
                    {
                        "name": "bar",
                        "type": "uint256"
                    }
                ],
                "outputs": [
                    {
                        "name": "baz",
                        "type": "uint256"
                    }
                ],
                "stateMutability": "view"
            }"#;

        serde_json::from_str::<FunctionAbiEntry>(json_str).unwrap();

        let json_str = r#"{
            "name": "foo",
            "inputs": [
                {
                    "name": "bar",
                    "type": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "baz",
                    "type": "uint256"
                }
            ]
        }"#;

        serde_json::from_str::<FunctionAbiEntry>(json_str).unwrap();
    }
}
