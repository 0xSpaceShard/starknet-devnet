use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::data_availability::L1DataAvailabilityMode;
use starknet_rs_core::types::{BlockId as ImportedBlockId, BlockTag as ImportedBlockTag, Felt};

use crate::contract_address::ContractAddress;
use crate::felt::BlockHash;
use crate::rpc::transactions::Transactions;
pub type BlockRoot = Felt;

#[derive(Clone, Debug)]
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
        #[derive(Copy, Clone, Debug, Deserialize)]
        enum BlockHashOrNumber {
            #[serde(rename = "block_hash")]
            Hash(Felt),
            #[serde(rename = "block_number")]
            Number(u64),
        }

        let value = serde_json::Value::deserialize(deserializer)?;
        match value.as_str() {
            Some("latest") => Ok(Self(ImportedBlockId::Tag(ImportedBlockTag::Latest))),
            Some("pre_confirmed") => Ok(Self(ImportedBlockId::Tag(ImportedBlockTag::PreConfirmed))),
            _ => match serde_json::from_value::<BlockHashOrNumber>(value) {
                Ok(BlockHashOrNumber::Hash(hash)) => Ok(Self(ImportedBlockId::Hash(hash))),
                Ok(BlockHashOrNumber::Number(n)) => Ok(Self(ImportedBlockId::Number(n))),
                Err(_) => Err(serde::de::Error::custom(
                    "Invalid block ID. Expected object with key (block_hash or block_number) or \
                     tag ('pre_confirmed' or 'latest').",
                )),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlockResult {
    Block(Block),
    PreConfirmedBlock(PreConfirmedBlock),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockStatus {
    /// Almost like pre-confirmed.
    PreConfirmed,
    /// A block that was created on L2.
    AcceptedOnL2,
    /// A block that was accepted on L1.
    AcceptedOnL1,
    /// A block rejected on L1.
    Rejected,
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
pub struct PreConfirmedBlock {
    #[serde(flatten)]
    pub header: PreConfirmedBlockHeader,
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
pub struct PreConfirmedBlockHeader {
    pub block_number: BlockNumber,
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
        let block_id = BlockId::deserialize(deserializer)?;
        Ok(match block_id {
            BlockId(ImportedBlockId::Hash(felt)) => Self::Hash(felt),
            BlockId(ImportedBlockId::Number(n)) => Self::Number(n),
            BlockId(ImportedBlockId::Tag(ImportedBlockTag::Latest)) => Self::Latest,
            BlockId(ImportedBlockId::Tag(ImportedBlockTag::PreConfirmed)) => {
                return Err(serde::de::Error::custom(
                    "Subscription block cannot be 'pre_confirmed'",
                ));
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockTag {
    PreConfirmed,
    Latest,
}

impl From<BlockTag> for starknet_rs_core::types::BlockTag {
    fn from(tag: BlockTag) -> Self {
        match tag {
            BlockTag::PreConfirmed => starknet_rs_core::types::BlockTag::PreConfirmed,
            BlockTag::Latest => starknet_rs_core::types::BlockTag::Latest,
        }
    }
}

#[cfg(test)]
mod test_subscription_block_id {
    use serde_json::json;

    use super::{BlockTag, SubscriptionBlockId};

    #[test]
    fn accept_latest() {
        serde_json::from_value::<SubscriptionBlockId>(json!("latest")).unwrap();
    }

    #[test]
    fn reject_pending_and_pre_confirmed() {
        serde_json::from_value::<SubscriptionBlockId>(json!("pending")).unwrap_err();
        serde_json::from_value::<SubscriptionBlockId>(json!("pre_confirmed")).unwrap_err();
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

    #[test]
    fn custom_block_tag_deserialization() {
        match serde_json::from_value::<BlockTag>(json!("latest")) {
            Ok(BlockTag::Latest) => (),
            other => panic!("Unexpected result: {other:?}"),
        }

        match serde_json::from_value::<BlockTag>(json!("pre_confirmed")) {
            Ok(BlockTag::PreConfirmed) => (),
            other => panic!("Unexpected result: {other:?}"),
        }
    }
}
