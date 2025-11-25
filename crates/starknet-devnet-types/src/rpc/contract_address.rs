use std::fmt::LowerHex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_rust::core::types::Felt;

use crate::error::DevnetResult;
use crate::patricia_key::{PATRICIA_KEY_ZERO, PatriciaKey};
use crate::serde_helpers::hex_string::{
    deserialize_to_prefixed_contract_address, serialize_contract_address_to_prefixed_hex,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractAddress(pub(crate) PatriciaKey);

impl ContractAddress {
    pub fn new(felt: Felt) -> DevnetResult<Self> {
        Ok(Self(PatriciaKey::new(felt)?))
    }

    /// Constructs a ContractAddress without range checks.
    /// Safety: `felt` must satisfy PatriciaKey bounds.
    pub fn new_unchecked(felt: Felt) -> Self {
        Self(PatriciaKey::new_unchecked(felt))
    }

    /// Constructs a zero address
    pub fn zero() -> Self {
        Self(PATRICIA_KEY_ZERO)
    }

    pub fn to_fixed_hex_string(&self) -> String {
        self.0.0.to_fixed_hex_string()
    }
}

impl Serialize for ContractAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_contract_address_to_prefixed_hex(self, serializer)
    }
}

impl<'de> Deserialize<'de> for ContractAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_to_prefixed_contract_address(deserializer)
    }
}

impl From<starknet_api::core::ContractAddress> for ContractAddress {
    fn from(value: starknet_api::core::ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl From<ContractAddress> for starknet_api::core::ContractAddress {
    fn from(value: ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl From<ContractAddress> for Felt {
    fn from(value: ContractAddress) -> Self {
        value.0.0
    }
}

impl LowerHex for ContractAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.0.to_hex_string().as_str())
    }
}
