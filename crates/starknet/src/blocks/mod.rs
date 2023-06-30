use std::collections::HashMap;

use starknet_api::{
    block::{BlockHeader, BlockNumber, BlockStatus},
    hash::{pedersen_hash_array, StarkFelt},
    stark_felt,
};
use starknet_types::{
    felt::{BlockHash, Felt},
    traits::HashProducer,
};

use crate::{traits::HashIdentified, transactions::Transaction};

pub(crate) struct StarknetBlocks {
    pub hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub pending_block: StarknetBlock,
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
            pending_block: StarknetBlock::create_pending_block(),
        }
    }
}

impl StarknetBlocks {
    pub fn insert(&mut self, block: StarknetBlock) {
        let block_number = block.header.block_number;
        self.hash_to_num.insert(block.block_hash(), block_number);
        self.num_to_block.insert(block_number, block);
    }
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct StarknetBlock {
    pub(crate) header: BlockHeader,
    transactions: Vec<Transaction>,
    pub(crate) status: BlockStatus,
}

impl StarknetBlock {
    pub(crate) fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }

    pub(crate) fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub(crate) fn block_hash(&self) -> BlockHash {
        self.header.block_hash.into()
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
            stark_felt!(self.header.block_number.0),     // block number
            self.header.state_root.0,                    // global_state_root
            *self.header.sequencer.0.key(),              // sequencer_address
            stark_felt!(self.header.timestamp.0),        // block_timestamp
            stark_felt!(self.transactions.len() as u64), // transaction_count
            stark_felt!(0_u8),                           // transaction_commitment
            stark_felt!(0_u8),                           // event_count
            stark_felt!(0_u8),                           // event_commitment
            stark_felt!(0_u8),                           // protocol_version
            stark_felt!(0_u8),                           // extra_data
            stark_felt!(self.header.parent_hash.0),      // parent_block_hash
        ]);

        Ok(Felt::from(hash))
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{BlockHeader, BlockNumber, BlockStatus};
    use starknet_types::traits::HashProducer;

    use crate::traits::HashIdentified;

    use super::{StarknetBlock, StarknetBlocks};

    #[test]
    fn correct_block_hash_computation() {
        assert!(false)
    }

    #[test]
    fn get_by_hash_is_correct() {
        let mut blocks = StarknetBlocks::default();
        let mut block_to_insert = StarknetBlock::create_pending_block();
        block_to_insert.header.block_hash = block_to_insert.generate_hash().unwrap().into();
        block_to_insert.header.block_number = BlockNumber(1);

        blocks.insert(block_to_insert.clone());

        let extracted_block = blocks.get_by_hash(block_to_insert.block_hash()).unwrap();
        assert!(block_to_insert == extracted_block.clone());
    }

    #[test]
    fn check_pending_block() {
        let block = StarknetBlock::create_pending_block();
        assert!(block.status == BlockStatus::Pending);
        assert!(block.transactions.is_empty());
        assert_eq!(block.header, BlockHeader::default());
    }
}
