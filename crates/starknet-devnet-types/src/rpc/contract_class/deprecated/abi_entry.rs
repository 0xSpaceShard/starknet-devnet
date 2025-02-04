use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
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

impl<'de> Deserialize<'de> for AbiEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = serde_json::Value::deserialize(deserializer)?;
        if raw_value.get("data").is_some() {
            let entry = serde_json::from_value(raw_value)
                .map_err(|e| serde::de::Error::custom(format!("Invalid event ABI entry: {e}")))?;
            Ok(AbiEntry::Event(entry))
        } else if raw_value.get("members").is_some() {
            let entry = serde_json::from_value(raw_value)
                .map_err(|e| serde::de::Error::custom(format!("Invalid struct ABI entry: {e}")))?;
            Ok(AbiEntry::Struct(entry))
        } else if raw_value.get("inputs").is_some() {
            let entry = serde_json::from_value(raw_value).map_err(|e| {
                serde::de::Error::custom(format!("Invalid function ABI entry: {e}"))
            })?;
            Ok(AbiEntry::Function(entry))
        } else {
            Err(serde::de::Error::custom(format!("Invalid ABI entry: {raw_value}")))
        }
    }
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
