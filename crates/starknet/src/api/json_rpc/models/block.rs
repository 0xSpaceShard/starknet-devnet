use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};

use super::{transaction::Transactions, ContractAddressHex, FeltHex};

pub type BlockHashHex = FeltHex;
pub type GlobalRootHex = FeltHex;

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
