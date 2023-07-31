use std::collections::HashMap;

use starknet_api::block::{BlockHeader, BlockNumber, BlockStatus, BlockTimestamp};
use starknet_api::hash::{pedersen_hash_array, StarkFelt};
use starknet_api::stark_felt;
use starknet_rs_core::types::BlockId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;

use crate::error::{self, Result};
use crate::state::state_diff::StateDiff;
use crate::state::StarknetState;
use crate::traits::HashIdentified;

pub(crate) struct StarknetBlocks {
    pub(crate) hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub(crate) num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub(crate) pending_block: StarknetBlock,
    pub(crate) last_block_hash: Option<BlockHash>,
    pub(crate) num_to_state_diff: HashMap<BlockNumber, StateDiff>,
    pub(crate) num_to_state: HashMap<BlockNumber, StarknetState>,
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
            num_to_state: HashMap::new(),
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

    pub fn save_state_at(&mut self, block_number: BlockNumber, state: StarknetState) {
        self.num_to_state.insert(block_number, state);
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
    pub fn get_blocks(
        &self,
        from: Option<BlockId>,
        to: Option<BlockId>,
    ) -> Result<Vec<&StarknetBlock>> {
        let starting_block = if let Some(block_id) = from {
            // If the value for block number provided is not correct it will return None
            // So we have to return an error
            let block_number =
                self.block_number_from_block_id(block_id).ok_or(error::Error::NoBlock)?;
            Some(block_number)
        } else {
            None
        };

        let ending_block = if let Some(block_id) = to {
            // if the value for block number provided is not correct it will return None
            // So we set the block number to the first possible block number which is 0
            let block_number =
                self.block_number_from_block_id(block_id).ok_or(error::Error::NoBlock)?;
            Some(block_number)
        } else {
            None
        };

        Ok(self
            .num_to_block
            .iter()
            .filter(|(current_block_number, _)| match (starting_block, ending_block) {
                (None, None) => true,
                (Some(start), None) => **current_block_number >= start,
                (None, Some(end)) => **current_block_number <= end,
                (Some(start), Some(end)) => {
                    **current_block_number >= start && **current_block_number <= end
                }
            })
            .map(|(_, block)| block)
            .collect())
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct StarknetBlock {
    pub(crate) header: BlockHeader,
    transaction_hashes: Vec<TransactionHash>,
    pub(crate) status: BlockStatus,
}

impl StarknetBlock {
    pub(crate) fn add_transaction(&mut self, transaction_hash: TransactionHash) {
        self.transaction_hashes.push(transaction_hash);
    }

    pub(crate) fn get_transactions(&self) -> &Vec<TransactionHash> {
        &self.transaction_hashes
    }

    pub fn status(&self) -> &BlockStatus {
        &self.status
    }

    pub fn block_hash(&self) -> BlockHash {
        self.header.block_hash.into()
    }

    pub fn parent_hash(&self) -> BlockHash {
        self.header.parent_hash.into()
    }

    pub fn sequencer_address(&self) -> ContractAddress {
        self.header.sequencer.into()
    }

    pub fn timestamp(&self) -> BlockTimestamp {
        self.header.timestamp
    }

    pub fn new_root(&self) -> Felt {
        self.header.state_root.0.into()
    }

    pub(crate) fn set_block_hash(&mut self, block_hash: BlockHash) {
        self.header.block_hash = block_hash.into();
    }

    pub fn block_number(&self) -> BlockNumber {
        self.header.block_number
    }

    pub(crate) fn create_pending_block() -> Self {
        Self {
            header: BlockHeader::default(),
            status: BlockStatus::Pending,
            transaction_hashes: Vec::new(),
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
            stark_felt!(self.transaction_hashes.len() as u64), // transaction_count
            stark_felt!(0_u8),                       // transaction_commitment
            stark_felt!(0_u8),                       // event_count
            stark_felt!(0_u8),                       // event_commitment
            stark_felt!(0_u8),                       // protocol_version
            stark_felt!(0_u8),                       // extra_data
            stark_felt!(self.header.parent_hash.0),  // parent_block_hash
        ]);

        Ok(Felt::from(hash))
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{BlockHash, BlockHeader, BlockNumber, BlockStatus};
    use starknet_rs_core::types::{BlockId, BlockTag};
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
            block_to_insert.header.block_hash = Felt::from(block_number as u128).into();
            blocks.insert(block_to_insert, StateDiff::default());
        }

        // check blocks len
        assert!(blocks.num_to_block.len() == 10);

        // 1. None, None
        // no filter
        assert_eq!(blocks.get_blocks(None, None).unwrap().len(), 10);

        // 2. Some, None
        assert_eq!(blocks.get_blocks(Some(BlockId::Number(9)), None).unwrap().len(), 3);
        // invalid from filter, should return err block not found
        assert!(blocks.get_blocks(Some(BlockId::Number(12)), None).is_err());
        // last block should be returned
        assert_eq!(blocks.get_blocks(Some(BlockId::Number(11)), None).unwrap().len(), 1);
        // from filter using hash
        assert_eq!(
            blocks.get_blocks(Some(BlockId::Hash(Felt::from(9).into())), None).unwrap().len(),
            3
        );
        // from filter using tag
        assert_eq!(blocks.get_blocks(Some(BlockId::Tag(BlockTag::Latest)), None).unwrap().len(), 1);
        assert_eq!(
            blocks.get_blocks(Some(BlockId::Tag(BlockTag::Pending)), None).unwrap().len(),
            1
        );

        // 3. None, Some
        // to filter using block number
        assert_eq!(blocks.get_blocks(None, Some(BlockId::Number(9))).unwrap().len(), 8);
        // to filter using invalid block number
        assert!(blocks.get_blocks(None, Some(BlockId::Number(0))).is_err());
        // to filter using hash
        assert_eq!(
            blocks.get_blocks(None, Some(BlockId::Hash(Felt::from(9).into()))).unwrap().len(),
            8
        );
        // to filter using invalid hash
        assert!(blocks.get_blocks(None, Some(BlockId::Hash(Felt::from(0).into()))).is_err());
        // to filter using tag
        assert_eq!(
            blocks.get_blocks(None, Some(BlockId::Tag(BlockTag::Latest))).unwrap().len(),
            10
        );
        assert_eq!(
            blocks.get_blocks(None, Some(BlockId::Tag(BlockTag::Pending))).unwrap().len(),
            10
        );
        // First block as to_block query param, should return empty collection
        assert_eq!(blocks.get_blocks(None, Some(BlockId::Number(2))).unwrap().len(), 1);
        // invalid to filter, should return err block not found
        assert!(blocks.get_blocks(None, Some(BlockId::Number(1))).is_err());

        // 4. Some, Some
        // from block number to block number
        assert_eq!(
            blocks.get_blocks(Some(BlockId::Number(2)), Some(BlockId::Number(9))).unwrap().len(),
            8
        );
        // from block number to to block hash
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(2)), Some(BlockId::Hash(Felt::from(9).into())))
                .unwrap()
                .len(),
            8
        );
        // from first block to latest/pending, should return all blocks
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(2)), Some(BlockId::Tag(BlockTag::Latest)))
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(2)), Some(BlockId::Tag(BlockTag::Pending)))
                .unwrap()
                .len(),
            10
        );

        // from last block to first block should return empty result
        assert!(
            blocks
                .get_blocks(Some(BlockId::Number(10)), Some(BlockId::Number(2)))
                .unwrap()
                .is_empty()
        );
        // from last block to latest/pending, should return 1 block
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(11)), Some(BlockId::Tag(BlockTag::Latest)))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(11)), Some(BlockId::Tag(BlockTag::Pending)))
                .unwrap()
                .len(),
            1
        );

        // bigger range than actual blocks in the collection, should return err
        assert!(blocks.get_blocks(Some(BlockId::Number(0)), Some(BlockId::Number(1000))).is_err());

        // from block hash to block_hash
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(2).into())),
                    Some(BlockId::Hash(Felt::from(9).into()))
                )
                .unwrap()
                .len(),
            8
        );
        assert!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(2).into())),
                    Some(BlockId::Hash(Felt::from(0).into()))
                )
                .is_err()
        );
        assert!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(10).into())),
                    Some(BlockId::Hash(Felt::from(5).into()))
                )
                .unwrap()
                .is_empty()
        );
        // from block hash to block number
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Hash(Felt::from(2).into())), Some(BlockId::Number(9)))
                .unwrap()
                .len(),
            8
        );
        // from last block hash to latest/pending
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(11).into())),
                    Some(BlockId::Tag(BlockTag::Latest))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(11).into())),
                    Some(BlockId::Tag(BlockTag::Pending))
                )
                .unwrap()
                .len(),
            1
        );

        // from tag to tag
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Latest)),
                    Some(BlockId::Tag(BlockTag::Latest))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Latest)),
                    Some(BlockId::Tag(BlockTag::Pending))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Pending)),
                    Some(BlockId::Tag(BlockTag::Latest))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Pending)),
                    Some(BlockId::Tag(BlockTag::Pending))
                )
                .unwrap()
                .len(),
            1
        );

        // from tag to block number/hash
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Tag(BlockTag::Latest)), Some(BlockId::Number(11)))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Latest)),
                    Some(BlockId::Hash(Felt::from(11).into()))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Tag(BlockTag::Pending)), Some(BlockId::Number(11)))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Pending)),
                    Some(BlockId::Hash(Felt::from(11).into()))
                )
                .unwrap()
                .len(),
            1
        );
        assert!(
            blocks
                .get_blocks(Some(BlockId::Tag(BlockTag::Latest)), Some(BlockId::Number(2)))
                .unwrap()
                .is_empty()
        );
        assert!(
            blocks
                .get_blocks(
                    Some(BlockId::Tag(BlockTag::Latest)),
                    Some(BlockId::Hash(Felt::from(2).into()))
                )
                .unwrap()
                .is_empty()
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
        assert!(block.transaction_hashes.is_empty());
        assert_eq!(block.header, BlockHeader::default());
    }
}
