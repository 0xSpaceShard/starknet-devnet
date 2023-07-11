pub(crate) mod abi_entry;
pub(crate) mod block;
pub(crate) mod contract_class;
pub(crate) mod state;
pub(crate) mod transaction;

use serde::{Deserialize, Serialize};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::starknet_api::block::BlockNumber;

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

impl From<BlockId> for starknet_rs_core::types::BlockId {
    fn from(block_id: BlockId) -> Self {
        type ImportedBlockId = starknet_rs_core::types::BlockId;
        type ImportedTag = starknet_rs_core::types::BlockTag;
        match block_id {
            BlockId::HashOrNumber(hash_or_number) => match hash_or_number {
                BlockHashOrNumber::Hash(hash) => ImportedBlockId::Hash(hash.0.into()),
                BlockHashOrNumber::Number(number) => ImportedBlockId::Number(number.0),
            },
            BlockId::Tag(tag) => match tag {
                Tag::Latest => ImportedBlockId::Tag(ImportedTag::Latest),
                Tag::Pending => ImportedBlockId::Tag(ImportedTag::Pending),
            },
        }
    }
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
