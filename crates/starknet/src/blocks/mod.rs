use std::collections::HashMap;

use starknet_api::block::{BlockHeader, BlockNumber, BlockStatus};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_api::stark_felt;
use starknet_types::felt::{BlockHash, Felt};
use starknet_types::traits::HashProducer;

use crate::traits::HashIdentified;
use crate::transactions::Transaction;

pub(crate) struct StarknetBlocks {
    pub(crate) hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub(crate) num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub(crate) pending_block: StarknetBlock,
    last_block_hash: Option<BlockHash>,
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
            last_block_hash: None,
        }
    }
}

impl StarknetBlocks {
    /// Inserts a block in the collection and modifies the block parent hash to match the last block
    /// hash
    pub fn insert(&mut self, mut block: StarknetBlock) {
        if self.last_block_hash.is_some() {
            block.header.parent_hash = self.last_block_hash.unwrap().into();
        }

        let hash = block.block_hash();
        let block_number = block.block_number();

        self.hash_to_num.insert(hash, block_number);
        self.num_to_block.insert(block_number, block);
        self.last_block_hash = Some(hash);
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

    pub(crate) fn set_block_hash(&mut self, block_hash: BlockHash) {
        self.header.block_hash = block_hash.into();
    }

    pub(crate) fn block_number(&self) -> BlockNumber {
        self.header.block_number
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
    use starknet_api::block::{BlockHash, BlockHeader, BlockNumber, BlockStatus};
    use starknet_types::traits::HashProducer;

    use super::{StarknetBlock, StarknetBlocks};
    use crate::traits::HashIdentified;

    #[test]
    fn block_hash_computation_doesnt_affect_internal_block_state() {
        let block = StarknetBlock::create_pending_block();
        assert!(block.generate_hash().unwrap() == block.generate_hash().unwrap());
    }

    #[test]
    fn correct_block_linking_via_parent_hash() {
        let mut blocks = StarknetBlocks::default();

        for block_number in 0..3 {
            let mut block = StarknetBlock::create_pending_block();

            block.status = BlockStatus::AcceptedOnL2;
            block.header.block_number = BlockNumber(block_number);
            block.set_block_hash(block.generate_hash().unwrap());

            blocks.insert(block);
        }

        assert!(
            blocks.num_to_block.get(&BlockNumber(0)).unwrap().header.parent_hash
                == BlockHash::default()
        );
        assert!(
            blocks.num_to_block.get(&BlockNumber(0)).unwrap().header.block_hash
                == blocks.num_to_block.get(&BlockNumber(1)).unwrap().header.parent_hash
        );
        assert!(
            blocks.num_to_block.get(&BlockNumber(1)).unwrap().header.block_hash
                == blocks.num_to_block.get(&BlockNumber(2)).unwrap().header.parent_hash
        );
        assert!(
            blocks.num_to_block.get(&BlockNumber(1)).unwrap().header.parent_hash
                != blocks.num_to_block.get(&BlockNumber(2)).unwrap().header.parent_hash
        )
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
