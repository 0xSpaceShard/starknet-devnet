use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};
use starknet_api::data_availability::L1DataAvailabilityMode;
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag, Felt};

use crate::contract_address::ContractAddress;
use crate::felt::BlockHash;
use crate::rpc::transactions::Transactions;
pub type BlockRoot = Felt;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum BlockHashOrNumber {
    #[serde(rename = "block_hash")]
    Hash(Felt),
    #[serde(rename = "block_number")]
    Number(u64),
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq))]
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
                BlockHashOrNumber::Hash(hash) => Ok(BlockId(ImportedBlockId::Hash(hash))),
                BlockHashOrNumber::Number(number) => Ok(BlockId(ImportedBlockId::Number(number))),
            }
        } else {
            Err(serde::de::Error::custom(format!("Invalid block ID: {value}")))
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlockResult {
    Block(Block),
    PendingBlock(PendingBlock),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct Block {
    pub status: BlockStatus,
    #[serde(flatten)]
    pub header: BlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct PendingBlock {
    #[serde(flatten)]
    pub header: PendingBlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct BlockHeader {
    pub block_hash: BlockHash,
    pub parent_hash: BlockHash,
    pub block_number: BlockNumber,
    pub sequencer_address: ContractAddress,
    pub new_root: BlockRoot,
    pub timestamp: BlockTimestamp,
    pub starknet_version: String,
    pub l1_gas_price: ResourcePrice,
    pub l2_gas_price: ResourcePrice,
    pub l1_data_gas_price: ResourcePrice,
    pub l1_da_mode: L1DataAvailabilityMode,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct PendingBlockHeader {
    pub parent_hash: BlockHash,
    pub sequencer_address: ContractAddress,
    pub timestamp: BlockTimestamp,
    pub starknet_version: String,
    pub l1_gas_price: ResourcePrice,
    pub l2_gas_price: ResourcePrice,
    pub l1_data_gas_price: ResourcePrice,
    pub l1_da_mode: L1DataAvailabilityMode,
}
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct ResourcePrice {
    // for now this will be always 0, this field is introduced in 0.5.0
    // but current version of blockifier/starknet_api doesn't return this value
    pub price_in_fri: Felt,
    pub price_in_wei: Felt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Data about reorganized blocks, starting and ending block number and hash
pub struct ReorgData {
    /// Hash of the first known block of the orphaned chain
    pub starting_block_hash: BlockHash,
    /// Number of the first known block of the orphaned chain
    pub starting_block_number: BlockNumber,
    /// The last known block of the orphaned chain
    pub ending_block_hash: BlockHash,
    /// Number of the last known block of the orphaned chain
    pub ending_block_number: BlockNumber,
}

#[derive(Debug, Clone)]
pub enum SubscriptionBlockId {
    Hash(Felt),
    Number(u64),
    Latest,
}

impl<'de> Deserialize<'de> for SubscriptionBlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let block_id = ImportedBlockId::deserialize(deserializer)?;
        Ok(match block_id {
            ImportedBlockId::Hash(felt) => Self::Hash(felt),
            ImportedBlockId::Number(n) => Self::Number(n),
            ImportedBlockId::Tag(ImportedBlockTag::Latest) => Self::Latest,
            ImportedBlockId::Tag(ImportedBlockTag::Pending) => {
                return Err(serde::de::Error::custom("Subscription block cannot be 'pending'"));
            }
        })
    }
}

impl From<SubscriptionBlockId> for ImportedBlockId {
    fn from(value: SubscriptionBlockId) -> Self {
        (&value).into()
    }
}

impl From<&SubscriptionBlockId> for ImportedBlockId {
    fn from(value: &SubscriptionBlockId) -> Self {
        match value {
            SubscriptionBlockId::Hash(hash) => Self::Hash(*hash),
            SubscriptionBlockId::Number(n) => Self::Number(*n),
            SubscriptionBlockId::Latest => Self::Tag(ImportedBlockTag::Latest),
        }
    }
}

#[cfg(test)]
mod test_subscription_block_id {
    use serde_json::json;

    use super::SubscriptionBlockId;

    #[test]
    fn accept_latest() {
        serde_json::from_value::<SubscriptionBlockId>(json!("latest")).unwrap();
    }

    #[test]
    fn reject_pending() {
        serde_json::from_value::<SubscriptionBlockId>(json!("pending")).unwrap_err();
    }

    #[test]
    fn reject_random_string() {
        serde_json::from_value::<SubscriptionBlockId>(json!("random string")).unwrap_err();
    }

    #[test]
    fn accept_valid_felt_as_block_hash() {
        serde_json::from_value::<SubscriptionBlockId>(json!({ "block_hash": "0x1" })).unwrap();
    }

    #[test]
    fn reject_invalid_felt_as_block_hash() {
        serde_json::from_value::<SubscriptionBlockId>(json!({ "block_hash": "invalid" }))
            .unwrap_err();
    }

    #[test]
    fn reject_unwrapped_felt_as_block_hash() {
        serde_json::from_value::<SubscriptionBlockId>(json!("0x123")).unwrap_err();
    }

    #[test]
    fn accept_valid_number_as_block_number() {
        serde_json::from_value::<SubscriptionBlockId>(json!({ "block_number": 123 })).unwrap();
    }

    #[test]
    fn reject_unwrapped_number_as_block_number() {
        serde_json::from_value::<SubscriptionBlockId>(json!(123)).unwrap_err();
    }
}
