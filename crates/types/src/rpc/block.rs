use serde::{Deserialize, Serialize};
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp};

use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, Felt};
use crate::rpc::transactions::Transactions;
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
