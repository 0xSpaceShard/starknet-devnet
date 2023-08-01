use serde::{Deserialize, Serialize};
use starknet_types::felt::Felt;
use starknet_types::starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};

use super::transaction::Transactions;
use super::ContractAddressHex;

pub type BlockHashHex = Felt;
pub type GlobalRootHex = Felt;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Block {
    pub status: BlockStatus,
    #[serde(flatten)]
    pub header: BlockHeader,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockHeader {
    pub block_hash: BlockHashHex,
    pub parent_hash: BlockHashHex,
    pub block_number: BlockNumber,
    pub sequencer_address: ContractAddressHex,
    pub new_root: GlobalRootHex,
    pub timestamp: BlockTimestamp,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct SyncStatus {
    pub starting_block_hash: BlockHashHex,
    pub starting_block_num: BlockNumber,
    pub current_block_hash: BlockHashHex,
    pub current_block_num: BlockNumber,
    pub highest_block_hash: BlockHashHex,
    pub highest_block_num: BlockNumber,
}

impl From<&starknet_core::StarknetBlock> for BlockHeader {
    fn from(value: &starknet_core::StarknetBlock) -> Self {
        Self {
            block_hash: value.block_hash(),
            parent_hash: value.parent_hash(),
            block_number: value.block_number(),
            sequencer_address: ContractAddressHex(value.sequencer_address()),
            new_root: value.new_root(),
            timestamp: value.timestamp(),
        }
    }
}
