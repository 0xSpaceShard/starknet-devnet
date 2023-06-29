use std::collections::HashMap;

use starknet_api::{block::{Block, BlockStatus, BlockNumber, BlockHeader}, hash::{pedersen_hash_array, StarkFelt}, stark_felt};
use starknet_types::{felt::{BlockHash, Felt}, traits::HashProducer};


use crate::{transactions::Transaction, traits::HashIdentified};

pub(crate) struct StarknetBlocks {
    pub hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub pending_block: Option<StarknetBlock>,
}

impl HashIdentified for StarknetBlocks {
    type Element = StarknetBlock;
    type Hash = BlockHash;

    fn get_by_hash(&self, hash: Self::Hash) -> Option<&Self::Element> {
        let block_number = self.hash_to_num.get(&hash)?;
        let block = self.num_to_block.get(block_number)?;

        Some(block)
    }
}

impl Default for StarknetBlocks {
    fn default() -> Self {
        Self {
            hash_to_num: HashMap::new(),
            num_to_block: HashMap::new(),
            pending_block: Some(StarknetBlock::create_pending_block()),
        }
    }
}

impl StarknetBlocks {
    pub(crate) fn add_pending_block(&mut self) {
        self.pending_block = Some(StarknetBlock::create_pending_block())
    }
}

pub(crate) struct StarknetBlock {
    header: BlockHeader,
    transactions: Vec<Transaction>,
    status: BlockStatus,
}

impl StarknetBlock {
    fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }

    pub(crate) fn create_pending_block() -> Self {
        Self {
            header: BlockHeader::default(),
            transactions: Vec::new(),
            status: BlockStatus::Pending,
        }
    }
}

impl HashProducer for StarknetBlock {
    fn generate_hash(&self) -> starknet_types::DevnetResult<BlockHash> {
        let hash = pedersen_hash_array(&[
            stark_felt!(self.header.block_number.0), // block number
            self.header.state_root.0,                // global_state_root
            *self.header.sequencer.0.key(),          // sequencer_address
            stark_felt!(self.header.timestamp.0),    // block_timestamp
            stark_felt!(self.transactions.len() as u64), // transaction_count
            stark_felt!(0_u8),                             // transaction_commitment
            stark_felt!(0_u8),                             // event_count
            stark_felt!(0_u8),                             // event_commitment
            stark_felt!(0_u8),                             // protocol_version
            stark_felt!(0_u8),                             // extra_data
            stark_felt!(self.header.parent_hash.0),             // parent_block_hash
        ]);

        Ok(Felt::from(hash))
    }
}