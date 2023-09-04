use serde::{Deserialize, Serialize};
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag};

use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, Felt};
use crate::rpc::transactions::Transactions;
pub type GlobalRootHex = Felt;

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tag {
    /// The most recent fully constructed block
    #[serde(rename = "latest")]
    Latest,
    /// Currently constructed block
    #[serde(rename = "pending")]
    Pending,
}

impl From<Tag> for ImportedBlockTag {
    fn from(value: Tag) -> Self {
        match value {
            Tag::Latest => ImportedBlockTag::Latest,
            Tag::Pending => ImportedBlockTag::Pending,
        }
    }
}

impl From<ImportedBlockTag> for Tag {
    fn from(value: ImportedBlockTag) -> Self {
        match value {
            ImportedBlockTag::Latest => Tag::Latest,
            ImportedBlockTag::Pending => Tag::Pending,
        }
    }
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
            BlockId::Tag(tag) => ImportedBlockId::Tag(tag.into()),
        }
    }
}

impl From<ImportedBlockId> for BlockId {
    fn from(block_id: ImportedBlockId) -> Self {
        match block_id {
            ImportedBlockId::Tag(tag) => BlockId::Tag(tag.into()),
            ImportedBlockId::Number(number) => {
                BlockId::HashOrNumber(BlockHashOrNumber::Number(BlockNumber(number)))
            }
            ImportedBlockId::Hash(hash) => {
                BlockId::HashOrNumber(BlockHashOrNumber::Hash(hash.into()))
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Block {
    pub status: BlockStatus,
    #[serde(flatten)]
    pub header: BlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockHeader {
    pub block_hash: BlockHash,
    pub parent_hash: BlockHash,
    pub block_number: BlockNumber,
    pub sequencer_address: ContractAddress,
    pub new_root: GlobalRootHex,
    pub timestamp: BlockTimestamp,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct SyncStatus {
    pub starting_block_hash: BlockHash,
    pub starting_block_num: BlockNumber,
    pub current_block_hash: BlockHash,
    pub current_block_num: BlockNumber,
    pub highest_block_hash: BlockHash,
    pub highest_block_num: BlockNumber,
}
