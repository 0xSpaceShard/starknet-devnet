pub(crate) mod state;

use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag};
use starknet_types::felt::Felt;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::serde_helpers::hex_string::{
    deserialize_to_prefixed_patricia_key, serialize_patricia_key_to_prefixed_hex,
};
use starknet_types::starknet_api::block::BlockNumber;

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
    Hash(Felt),
    #[serde(rename = "block_number")]
    Number(BlockNumber),
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BlockId {
    HashOrNumber(BlockHashOrNumber),
    Tag(Tag),
}

impl From<BlockId> for ImportedBlockId {
    fn from(block_id: BlockId) -> Self {
        match block_id {
            BlockId::HashOrNumber(hash_or_number) => match hash_or_number {
                BlockHashOrNumber::Hash(hash) => ImportedBlockId::Hash(hash.into()),
                BlockHashOrNumber::Number(number) => ImportedBlockId::Number(number.0),
            },
            BlockId::Tag(tag) => match tag {
                Tag::Latest => ImportedBlockId::Tag(ImportedBlockTag::Latest),
                Tag::Pending => ImportedBlockId::Tag(ImportedBlockTag::Pending),
            },
        }
    }
}

/// Patricia key serialized/deserialized from/to prefixed hex string
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatriciaKeyHex(
    #[serde(
        serialize_with = "serialize_patricia_key_to_prefixed_hex",
        deserialize_with = "deserialize_to_prefixed_patricia_key"
    )]
    pub PatriciaKey,
);
