pub(crate) mod state;

use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag};
use starknet_types::felt::Felt;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{BlockHashOrNumber, BlockId, Tag};
use starknet_types::serde_helpers::hex_string::{
    deserialize_to_prefixed_patricia_key, serialize_patricia_key_to_prefixed_hex,
};
use starknet_types::starknet_api::block::BlockNumber;

/// Patricia key serialized/deserialized from/to prefixed hex string
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatriciaKeyHex(
    #[serde(
        serialize_with = "serialize_patricia_key_to_prefixed_hex",
        deserialize_with = "deserialize_to_prefixed_patricia_key"
    )]
    pub PatriciaKey,
);
