use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};
use starknet_api::data_availability::L1DataAvailabilityMode;
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag};

use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, Felt};
use crate::rpc::transactions::Transactions;
pub type GlobalRootHex = Felt;

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockHashOrNumber {
    #[serde(rename = "block_hash")]
    Hash(Felt),
    #[serde(rename = "block_number")]
    Number(u64),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BlockId(pub ImportedBlockId);

impl From<ImportedBlockId> for BlockId {
    fn from(value: ImportedBlockId) -> Self {
        Self(value)
    }
}

impl AsRef<ImportedBlockId> for BlockId {
    fn as_ref(&self) -> &ImportedBlockId {
        &self.0
    }
}

impl From<BlockId> for ImportedBlockId {
    fn from(block_id: BlockId) -> Self {
        block_id.0
    }
}

impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.as_str().is_some() {
            let block_tag: ImportedBlockTag = serde_json::from_value(value)
                .map_err(|e| serde::de::Error::custom(format!("Invalid block ID: {e}")))?;
            Ok(BlockId(ImportedBlockId::Tag(block_tag)))
        } else if value.as_object().is_some() {
            let block_id: BlockHashOrNumber = serde_json::from_value(value)
                .map_err(|e| serde::de::Error::custom(format!("Invalid block ID: {e}")))?;
            match block_id {
                BlockHashOrNumber::Hash(hash) => Ok(BlockId(ImportedBlockId::Hash(hash.into()))),
                BlockHashOrNumber::Number(number) => Ok(BlockId(ImportedBlockId::Number(number))),
            }
        } else {
            Err(serde::de::Error::custom(format!("Invalid block ID: {value}")))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub status: BlockStatus,
    #[serde(flatten)]
    pub header: BlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingBlock {
    #[serde(flatten)]
    pub header: PendingBlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockHeader {
    pub block_hash: BlockHash,
    pub parent_hash: BlockHash,
    pub block_number: BlockNumber,
    pub sequencer_address: ContractAddress,
    pub new_root: GlobalRootHex,
    pub timestamp: BlockTimestamp,
    pub starknet_version: String,
    pub l1_gas_price: ResourcePrice,
    pub l1_data_gas_price: ResourcePrice,
    pub l1_da_mode: L1DataAvailabilityMode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingBlockHeader {
    pub parent_hash: BlockHash,
    pub sequencer_address: ContractAddress,
    pub new_root: GlobalRootHex,
    pub timestamp: BlockTimestamp,
    pub starknet_version: String,
    pub l1_gas_price: ResourcePrice,
    pub l1_data_gas_price: ResourcePrice,
    pub l1_da_mode: L1DataAvailabilityMode,
}
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePrice {
    // for now this will be always 0, this field is introduced in 0.5.0
    // but current version of blockifier/starknet_api doesnt return this value
    pub price_in_fri: Felt,
    pub price_in_wei: Felt,
}
