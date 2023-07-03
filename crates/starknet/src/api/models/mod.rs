pub(crate) mod abi_entry;
pub(crate) mod block;
pub(crate) mod contract_class;
pub(crate) mod state;
pub(crate) mod transaction;

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use starknet_types::{
    contract_address::ContractAddress, felt::Felt, patricia_key::PatriciaKey,
    starknet_api::block::BlockNumber, traits::ToHexString,
};

use super::serde_helpers::hex_string::{
    deserialize_prefixed_hex_string_to_felt, deserialize_to_prefixed_contract_address,
    deserialize_to_prefixed_patricia_key, serialize_contract_address_to_prefixed_hex,
    serialize_patricia_key_to_prefixed_hex, serialize_to_prefixed_hex,
};

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tag {
    /// The most recent fully constructed block
    #[serde(rename = "latest")]
    Latest,
    /// Currently constructed block
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockHashOrNumber {
    #[serde(rename = "block_hash")]
    Hash(FeltHex),
    #[serde(rename = "block_number")]
    Number(BlockNumber),
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BlockId {
    HashOrNumber(BlockHashOrNumber),
    Tag(Tag),
}

/// Felt serialized/deserialized from/to prefixed hex string
#[derive(Debug, Default, Hash, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeltHex(
    #[serde(
        serialize_with = "serialize_to_prefixed_hex",
        deserialize_with = "deserialize_prefixed_hex_string_to_felt"
    )]
    pub Felt,
);

impl Display for FeltHex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.to_prefixed_hex_str().as_str())
    }
}

/// Contract address serialized/deserialized from/to prefixed hex string
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractAddressHex(
    #[serde(
        serialize_with = "serialize_contract_address_to_prefixed_hex",
        deserialize_with = "deserialize_to_prefixed_contract_address"
    )]
    pub ContractAddress,
);

/// Patricia key serialized/deserialized from/to prefixed hex string
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatriciaKeyHex(
    #[serde(
        serialize_with = "serialize_patricia_key_to_prefixed_hex",
        deserialize_with = "deserialize_to_prefixed_patricia_key"
    )]
    pub PatriciaKey,
);
