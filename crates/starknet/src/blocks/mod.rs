use std::collections::HashMap;

use starknet_api::block::{BlockHeader, BlockNumber, BlockStatus};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_api::stark_felt;
use starknet_rs_core::types::BlockId;
use starknet_types::felt::{BlockHash, Felt};
use starknet_types::traits::HashProducer;

use crate::state::state_diff::StateDiff;
use crate::traits::HashIdentified;
use crate::transactions::Transaction;
use crate::error::Result;

pub(crate) struct StarknetBlocks {
    pub(crate) hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub(crate) num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub(crate) pending_block: StarknetBlock,
    pub(crate) last_block_hash: Option<BlockHash>,
    pub(crate) num_to_state_diff: HashMap<BlockNumber, StateDiff>,
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
            num_to_state_diff: HashMap::new(),
        }
    }
}

impl StarknetBlocks {
    /// Inserts a block in the collection and modifies the block parent hash to match the last block
    /// hash
    pub fn insert(&mut self, mut block: StarknetBlock, state_diff: StateDiff) {
        if self.last_block_hash.is_some() {
            block.header.parent_hash = self.last_block_hash.unwrap().into();
        }

        let hash = block.block_hash();
        let block_number = block.block_number();

        self.hash_to_num.insert(hash, block_number);
        self.num_to_block.insert(block_number, block);
        self.num_to_state_diff.insert(block_number, state_diff);
        self.last_block_hash = Some(hash);
    }

    pub fn get_by_block_id(&self, block_id: BlockId) -> Option<&StarknetBlock> {
        match block_id {
            BlockId::Hash(hash) => self.get_by_hash(Felt::from(hash)),
            BlockId::Number(block_number) => self.num_to_block.get(&BlockNumber(block_number)),
            // latest and pending for now will return the latest one
            BlockId::Tag(_) => {
                if let Some(hash) = self.last_block_hash {
                    self.get_by_hash(hash)
                } else {
                    None
                }
            }
        }
    }

    /// Returns the block number from a block id, by finding the block by the block id
    fn block_number_from_block_id(&self, block_id: BlockId) -> Option<BlockNumber> {
        self.get_by_block_id(block_id).map(|block| block.block_number())
    }

    /// filter blocks based on from and to block ids
    pub fn get_blocks(&self, from: Option<BlockId>, to: Option<BlockId>) -> Vec<StarknetBlock> {
        let starting_block = if let Some(block_id) = from {
            // if the value for block number provided is not correct it will return None
            // so we have to handle the None value, because in the filter below it will
            // skip check for this filter value. So we set it to be after the last block or 1 if
            // there is no last block
            match self.block_number_from_block_id(block_id) {
                Some(block_number) => Some(block_number),
                None => {
                    if let Some(last_block_hash) = self.last_block_hash {
                        // we are sure that if there is a last_block_hash it will have a block
                        // number
                        Some(
                            self.hash_to_num
                                .get(&last_block_hash)
                                .copied()
                                .unwrap_or_default()
                                .next(),
                        )
                    } else {
                        Some(BlockNumber(1))
                    }
                }
            }
        } else {
            None
        };

        let ending_block = if let Some(block_id) = to {
            // if the value for block number provided is not correct it will return None
            // So we set the block number to the first possible block number which is 0
            match self.block_number_from_block_id(block_id) {
                Some(block_number) => Some(block_number),
                None => Some(BlockNumber(0)),
            }
        } else {
            None
        };

        self.num_to_block
            .iter()
            .filter(|(current_block_number, _)| match (starting_block, ending_block) {
                (None, None) => true,
                (None, Some(end)) => **current_block_number < end,
                (Some(start), None) => **current_block_number >= start,
                (Some(start), Some(end)) => {
                    **current_block_number >= start && **current_block_number < end
                }
            })
            .map(|(_, block)| block)
            .cloned()
            .collect()
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
    use starknet_rs_core::types::BlockId;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use super::{StarknetBlock, StarknetBlocks};
    use crate::state::state_diff::StateDiff;
    use crate::traits::HashIdentified;

    #[test]
    fn block_number_from_block_id_should_return_correct_result() {
        let mut blocks = StarknetBlocks::default();
        let mut block_to_insert = StarknetBlock::create_pending_block();

        // latest/pending block returns none, because collection is empty
        assert!(
            blocks
                .block_number_from_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
                .is_none()
        );
        assert!(
            blocks
                .block_number_from_block_id(BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Pending
                ))
                .is_none()
        );

        let block_hash = block_to_insert.generate_hash().unwrap();
        block_to_insert.header.block_number = BlockNumber(10);
        block_to_insert.header.block_hash = block_hash.into();

        blocks.insert(block_to_insert, StateDiff::default());

        // returns block number, even if the block number is not present in the collection
        assert!(blocks.block_number_from_block_id(BlockId::Number(11)).is_none());
        assert!(blocks.block_number_from_block_id(BlockId::Number(10)).is_some());
        // returns none because there is no block with the given hash
        assert!(blocks.block_number_from_block_id(BlockId::Hash(Felt::from(1).into())).is_none());
        assert!(
            blocks
                .block_number_from_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
                .is_some()
        );
        assert!(
            blocks
                .block_number_from_block_id(BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Pending
                ))
                .is_some()
        );
        assert!(blocks.block_number_from_block_id(BlockId::Hash(block_hash.into())).is_some());
    }

    #[test]
    fn get_blocks_with_filter() {
        let mut blocks = StarknetBlocks::default();

        for block_number in 2..12 {
            let mut block_to_insert = StarknetBlock::create_pending_block();
            block_to_insert.header.block_number = BlockNumber(block_number);
            block_to_insert.header.block_hash = block_to_insert.generate_hash().unwrap().into();
            blocks.insert(block_to_insert, StateDiff::default());
        }

        // check blocks len
        assert!(blocks.num_to_block.len() == 10);
        // only from filter
        assert_eq!(blocks.get_blocks(Some(BlockId::Number(9)), None).len(), 3);
        // no filter
        assert_eq!(blocks.get_blocks(None, None).len(), 10);
        // invalid from filter, should return no blocks
        assert!(blocks.get_blocks(Some(BlockId::Number(12)), None).is_empty());
        // invalid from filter, should return no blocks
        assert!(blocks.get_blocks(Some(BlockId::Number(13)), None).is_empty());
        // invalid to filter, should return no blocks
        assert!(blocks.get_blocks(None, Some(BlockId::Number(1))).is_empty());
        // invalid to filter, should return no blocks
        assert!(blocks.get_blocks(None, Some(BlockId::Number(2))).is_empty());
        // bigger range than actual blocks in the collection
        assert_eq!(
            blocks.get_blocks(Some(BlockId::Number(0)), Some(BlockId::Number(1000))).len(),
            10
        );
    }

    #[test]
    fn get_by_block_id_is_correct() {
        let mut blocks = StarknetBlocks::default();
        let mut block_to_insert = StarknetBlock::create_pending_block();
        block_to_insert.header.block_hash = block_to_insert.generate_hash().unwrap().into();
        block_to_insert.header.block_number = BlockNumber(10);

        blocks.insert(block_to_insert.clone(), StateDiff::default());

        let extracted_block = blocks.get_by_block_id(BlockId::Number(10)).unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block =
            blocks.get_by_block_id(BlockId::Hash(block_to_insert.block_hash().into())).unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block = blocks
            .get_by_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
            .unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block = blocks
            .get_by_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Pending))
            .unwrap();
        assert!(block_to_insert == extracted_block.clone());

        match blocks.get_by_block_id(BlockId::Number(11)) {
            None => (),
            _ => panic!("Expected none"),
        }
    }

    #[test]
    fn correct_block_linking_via_parent_hash() {
        let mut blocks = StarknetBlocks::default();

        for block_number in 0..3 {
            let mut block = StarknetBlock::create_pending_block();

            block.status = BlockStatus::AcceptedOnL2;
            block.header.block_number = BlockNumber(block_number);
            block.set_block_hash(block.generate_hash().unwrap());

            blocks.insert(block, StateDiff::default());
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

        blocks.insert(block_to_insert.clone(), StateDiff::default());

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
