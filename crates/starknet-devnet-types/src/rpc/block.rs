use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::core::{
    EventCommitment, ReceiptCommitment, StateDiffCommitment, TransactionCommitment,
};
use starknet_api::data_availability::L1DataAvailabilityMode;
use starknet_rust::core::types::Felt;

use crate::contract_address::ContractAddress;
use crate::felt::BlockHash;
use crate::rpc::transactions::Transactions;
pub type BlockRoot = Felt;

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum BlockId {
    /// Block hash.
    Hash(Felt),
    /// Block number (height).
    Number(u64),
    /// Block tag
    Tag(BlockTag),
}

impl From<BlockId> for starknet_rust::core::types::BlockId {
    fn from(block_id: BlockId) -> Self {
        match block_id {
            BlockId::Hash(felt) => Self::Hash(felt),
            BlockId::Number(n) => Self::Number(n),
            BlockId::Tag(tag) => Self::Tag(tag.into()),
        }
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
            Some("latest") => Ok(Self::Tag(BlockTag::Latest)),
            Some("pre_confirmed") => Ok(Self::Tag(BlockTag::PreConfirmed)),
            Some("l1_accepted") => Ok(Self::Tag(BlockTag::L1Accepted)),
            _ => match serde_json::from_value::<BlockHashOrNumber>(value) {
                Ok(BlockHashOrNumber::Hash(hash)) => Ok(Self::Hash(hash)),
                Ok(BlockHashOrNumber::Number(n)) => Ok(Self::Number(n)),
                Err(_) => Err(serde::de::Error::custom(
                    "Invalid block ID. Expected object with key (block_hash or block_number) or \
                     tag ('pre_confirmed' or 'latest' or 'l1_accepted').",
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
    pub state_diff_commitment: StateDiffCommitment,
    pub state_diff_length: u64,
    pub transaction_commitment: TransactionCommitment,
    pub event_commitment: EventCommitment,
    #[serde(rename = "transaction_count")]
    pub n_transactions: u64,
    #[serde(rename = "event_count")]
    pub n_events: u64,
    pub receipt_commitment: ReceiptCommitment,
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

impl From<starknet_rust::core::types::ResourcePrice> for ResourcePrice {
    fn from(value: starknet_rust::core::types::ResourcePrice) -> Self {
        Self { price_in_fri: value.price_in_fri, price_in_wei: value.price_in_wei }
    }
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
    L1Accepted,
}

impl<'de> Deserialize<'de> for SubscriptionBlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let block_id = BlockId::deserialize(deserializer)?;
        match block_id {
            BlockId::Hash(felt) => Ok(Self::Hash(felt)),
            BlockId::Number(n) => Ok(Self::Number(n)),
            BlockId::Tag(BlockTag::Latest) => Ok(Self::Latest),
            BlockId::Tag(BlockTag::PreConfirmed) => {
                Err(serde::de::Error::custom("Subscription block cannot be 'pre_confirmed'"))
            }
            BlockId::Tag(BlockTag::L1Accepted) => {
                Err(serde::de::Error::custom("Subscription block cannot be 'l1_accepted'"))
            }
        }
    }
}

impl From<SubscriptionBlockId> for BlockId {
    fn from(block_id: SubscriptionBlockId) -> Self {
        (&block_id).into()
    }
}

impl From<&SubscriptionBlockId> for BlockId {
    fn from(value: &SubscriptionBlockId) -> Self {
        match value {
            SubscriptionBlockId::Hash(hash) => Self::Hash(*hash),
            SubscriptionBlockId::Number(n) => Self::Number(*n),
            SubscriptionBlockId::Latest => Self::Tag(BlockTag::Latest),
            SubscriptionBlockId::L1Accepted => Self::Tag(BlockTag::L1Accepted),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockTag {
    PreConfirmed,
    Latest,
    L1Accepted,
}

impl From<BlockTag> for starknet_rust::core::types::BlockTag {
    fn from(tag: BlockTag) -> Self {
        match tag {
            BlockTag::PreConfirmed => Self::PreConfirmed,
            BlockTag::Latest => Self::Latest,
            BlockTag::L1Accepted => Self::L1Accepted,
        }
    }
}

#[cfg(test)]
mod test_block_id {
    use serde_json::json;
    use starknet_rust::core::types::Felt;

    use super::BlockTag;
    use crate::rpc::block::BlockId;

    #[test]
    fn custom_block_id_deserialization() {
        for (raw, expected) in [
            (r#"{"block_hash": "0x1"}"#, BlockId::Hash(Felt::ONE)),
            (r#"{"block_number": 123}"#, BlockId::Number(123)),
            (r#""latest""#, BlockId::Tag(BlockTag::Latest)),
            (r#""pre_confirmed""#, BlockId::Tag(BlockTag::PreConfirmed)),
            (r#""l1_accepted""#, BlockId::Tag(BlockTag::L1Accepted)),
        ] {
            assert_eq!(serde_json::from_str::<BlockId>(raw).unwrap(), expected);
        }
    }

    #[test]
    fn custom_block_tag_deserialization() {
        for (raw, expected) in [
            ("latest", BlockTag::Latest),
            ("pre_confirmed", BlockTag::PreConfirmed),
            ("l1_accepted", BlockTag::L1Accepted),
        ] {
            assert_eq!(serde_json::from_value::<BlockTag>(json!(raw)).unwrap(), expected);
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
    fn reject_non_latest_subscription_block_tag() {
        for tag in ["pending", "pre_confirmed", "l1_accepted"] {
            serde_json::from_value::<SubscriptionBlockId>(json!(tag)).unwrap_err();
        }
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
