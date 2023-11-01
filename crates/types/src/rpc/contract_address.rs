use std::fmt::LowerHex;

use cairo_felt::Felt252;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_rs_ff::FieldElement;

use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use crate::patricia_key::PatriciaKey;
use crate::serde_helpers::hex_string::{
    deserialize_to_prefixed_contract_address, serialize_contract_address_to_prefixed_hex,
};
use crate::traits::ToHexString;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractAddress(pub(crate) PatriciaKey);

impl ContractAddress {
    pub fn new(felt: Felt) -> DevnetResult<Self> {
        Ok(Self(PatriciaKey::new(felt)?))
    }

    /// Constructs a zero address
    pub fn zero() -> Self {
        // using unwrap because we are sure it works for 0x0
        Self::new(Felt::from(0)).unwrap()
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

impl TryFrom<ContractAddress> for starknet_api::core::ContractAddress {
    type Error = Error;

    fn try_from(value: ContractAddress) -> DevnetResult<Self> {
        let patricia_key: starknet_api::core::PatriciaKey = value.0.try_into()?;
        Ok(starknet_api::core::ContractAddress(patricia_key))
    }
}

impl From<ContractAddress> for Felt252 {
    fn from(value: ContractAddress) -> Self {
        Felt::from(value).into()
    }
}

impl From<ContractAddress> for FieldElement {
    fn from(value: ContractAddress) -> Self {
        FieldElement::from(value.0.0)
    }
}

impl ToHexString for ContractAddress {
    fn to_prefixed_hex_str(&self) -> String {
        self.0.0.to_prefixed_hex_str()
    }

    fn to_nonprefixed_hex_str(&self) -> String {
        self.0.0.to_nonprefixed_hex_str()
    }
}

impl LowerHex for ContractAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.0.to_prefixed_hex_str().as_str())
    }
}
