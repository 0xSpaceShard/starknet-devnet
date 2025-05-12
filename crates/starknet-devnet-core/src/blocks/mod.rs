use std::collections::HashMap;

use indexmap::IndexMap;
use starknet_api::block::{
    BlockHeader, BlockHeaderWithoutHash, BlockNumber, BlockStatus, BlockTimestamp,
};
use starknet_api::data_availability::L1DataAvailabilityMode;
use starknet_api::felt;
use starknet_rs_core::types::{BlockId, BlockTag, Felt};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, TransactionHash};
use starknet_types::rpc::block::{
    BlockHeader as TypesBlockHeader, PendingBlockHeader as TypesPendingBlockHeader, ResourcePrice,
};
use starknet_types::traits::HashProducer;
use starknet_types_core::hash::{Pedersen, StarkHash};

use crate::constants::{DEVNET_DEFAULT_STARTING_BLOCK_NUMBER, STARKNET_VERSION};
use crate::error::{DevnetResult, Error};
use crate::state::StarknetState;
use crate::state::state_diff::StateDiff;
use crate::traits::HashIdentified;

pub(crate) struct StarknetBlocks {
    pub(crate) num_to_hash: IndexMap<BlockNumber, BlockHash>,
    pub(crate) hash_to_block: HashMap<BlockHash, StarknetBlock>,
    pub(crate) pending_block: StarknetBlock,
    pub(crate) last_block_hash: Option<BlockHash>,
    pub(crate) hash_to_state_diff: HashMap<BlockHash, StateDiff>,
    pub(crate) hash_to_state: HashMap<BlockHash, StarknetState>,
    pub(crate) aborted_blocks: Vec<Felt>,
    pub(crate) starting_block_number: u64,
}

impl HashIdentified for StarknetBlocks {
    type Element = StarknetBlock;
    type Hash = BlockHash;

    fn get_by_hash(&self, hash: Self::Hash) -> Option<&Self::Element> {
        let block = self.hash_to_block.get(&hash)?;

        Some(block)
    }
}

impl Default for StarknetBlocks {
    fn default() -> Self {
        Self {
            num_to_hash: IndexMap::new(),
            hash_to_block: HashMap::new(),
            pending_block: StarknetBlock::create_pending_block(),
            last_block_hash: None,
            hash_to_state_diff: HashMap::new(),
            hash_to_state: HashMap::new(),
            aborted_blocks: Vec::new(),
            starting_block_number: DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        }
    }
}

impl StarknetBlocks {
    pub fn new(starting_block_number: u64, last_block_hash: Option<Felt>) -> Self {
        let mut blocks = Self { starting_block_number, ..Default::default() };
        blocks.pending_block.set_block_number(starting_block_number);
        blocks.last_block_hash = last_block_hash;
        blocks
    }

    /// Inserts a block in the collection and modifies the block parent hash to match the last block
    /// hash
    pub fn insert(&mut self, mut block: StarknetBlock, state_diff: StateDiff) {
        if let Some(last_block_hash) = self.last_block_hash {
            block.header.block_header_without_hash.parent_hash =
                starknet_api::block::BlockHash(last_block_hash);
        }

        let hash = block.block_hash();
        let block_number = block.block_number();

        self.num_to_hash.insert(block_number, hash);
        self.hash_to_block.insert(hash, block);
        self.hash_to_state_diff.insert(hash, state_diff);
        self.last_block_hash = Some(hash);
    }

    fn get_by_num(&self, num: &BlockNumber) -> Option<&StarknetBlock> {
        let block_hash = self.num_to_hash.get(num)?;
        let block = self.hash_to_block.get(block_hash)?;

        Some(block)
    }

    pub fn save_state_at(&mut self, block_hash: Felt, state: StarknetState) {
        self.hash_to_state.insert(block_hash, state);
    }

    fn get_by_latest_hash(&self) -> Option<&StarknetBlock> {
        if let Some(hash) = self.last_block_hash { self.get_by_hash(hash) } else { None }
    }

    pub fn get_by_block_id(&self, block_id: &BlockId) -> Option<&StarknetBlock> {
        match block_id {
            BlockId::Hash(hash) => self.get_by_hash(*hash),
            BlockId::Number(block_number) => self.get_by_num(&BlockNumber(*block_number)),
            BlockId::Tag(BlockTag::Pending) => Some(&self.pending_block),
            BlockId::Tag(BlockTag::Latest) => self.get_by_latest_hash(),
        }
    }

    /// Returns the block number from a block id, by finding the block by the block id
    fn block_number_from_block_id(&self, block_id: &BlockId) -> Option<BlockNumber> {
        self.get_by_block_id(block_id).map(|block| block.block_number())
    }

    /// Filter blocks based on from and to block ids and returns a collection of block's references
    /// in ascending order
    ///
    /// # Arguments
    /// * `from` - The block id from which to start the filtering
    /// * `to` - The block id to which to end the filtering
    pub fn get_blocks(
        &self,
        from: Option<BlockId>,
        to: Option<BlockId>,
    ) -> DevnetResult<Vec<&StarknetBlock>> {
        // used IndexMap to keep elements in the order of the keys
        let mut filtered_blocks: IndexMap<Felt, &StarknetBlock> = IndexMap::new();

        let pending_block_number = self.pending_block.block_number();

        let starting_block = if let Some(block_id) = from {
            // If the value for block number provided is not correct it will return None
            // So we have to return an error
            let block_number = self.block_number_from_block_id(&block_id).ok_or(Error::NoBlock)?;
            Some(block_number)
        } else {
            None
        };

        let ending_block = if let Some(block_id) = to {
            // if the value for block number provided is not correct it will return None
            // So we set the block number to the first possible block number which is 0
            let block_number = self.block_number_from_block_id(&block_id).ok_or(Error::NoBlock)?;
            Some(block_number)
        } else {
            None
        };

        fn is_block_number_in_range(
            current_block_number: BlockNumber,
            starting_block: Option<BlockNumber>,
            ending_block: Option<BlockNumber>,
        ) -> bool {
            match (starting_block, ending_block) {
                (None, None) => true,
                (Some(start), None) => current_block_number >= start,
                (None, Some(end)) => current_block_number <= end,
                (Some(start), Some(end)) => {
                    current_block_number >= start && current_block_number <= end
                }
            }
        }

        let mut insert_pending_block_in_final_result = true;
        // iterate over the blocks and apply the filter
        // then insert the filtered blocks into the index map
        self.num_to_hash
            .iter()
            .filter(|(current_block_number, _)| {
                is_block_number_in_range(**current_block_number, starting_block, ending_block)
            })
            .for_each(|(block_number, block_hash)| {
                if *block_number == pending_block_number {
                    insert_pending_block_in_final_result = false;
                }
                filtered_blocks.insert(*block_hash, &self.hash_to_block[block_hash]);
            });

        let mut result: Vec<&StarknetBlock> = filtered_blocks.into_values().collect();

        if is_block_number_in_range(pending_block_number, starting_block, ending_block)
            && insert_pending_block_in_final_result
        {
            result.push(&self.pending_block);
        }

        Ok(result)
    }

    pub fn next_block_number(&self) -> BlockNumber {
        BlockNumber(self.pending_block.block_number().0 - self.aborted_blocks.len() as u64)
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct StarknetBlock {
    pub(crate) header: BlockHeader,
    transaction_hashes: Vec<TransactionHash>,
    pub(crate) status: BlockStatus,
}

impl From<&StarknetBlock> for TypesPendingBlockHeader {
    fn from(value: &StarknetBlock) -> Self {
        Self {
            parent_hash: value.parent_hash(),
            sequencer_address: value.sequencer_address(),
            timestamp: value.timestamp(),
            starknet_version: STARKNET_VERSION.to_string(),
            l1_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l1_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l1_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
            l1_data_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l1_data_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l1_data_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
            l1_da_mode: value.header.block_header_without_hash.l1_da_mode,
            l2_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l2_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l2_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
        }
    }
}

impl From<&StarknetBlock> for TypesBlockHeader {
    fn from(value: &StarknetBlock) -> Self {
        Self {
            block_hash: value.block_hash(),
            parent_hash: value.parent_hash(),
            block_number: value.block_number(),
            sequencer_address: value.sequencer_address(),
            new_root: value.new_root(),
            timestamp: value.timestamp(),
            starknet_version: STARKNET_VERSION.to_string(),
            l1_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l1_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l1_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
            l1_data_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l1_data_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l1_data_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
            l1_da_mode: value.header.block_header_without_hash.l1_da_mode,
            l2_gas_price: ResourcePrice {
                price_in_fri: value
                    .header
                    .block_header_without_hash
                    .l2_gas_price
                    .price_in_fri
                    .0
                    .into(),
                price_in_wei: value
                    .header
                    .block_header_without_hash
                    .l2_gas_price
                    .price_in_wei
                    .0
                    .into(),
            },
        }
    }
}

impl StarknetBlock {
    pub(crate) fn add_transaction(&mut self, transaction_hash: TransactionHash) {
        self.transaction_hashes.push(transaction_hash);
    }

    pub fn get_transactions(&self) -> &Vec<TransactionHash> {
        &self.transaction_hashes
    }

    pub fn status(&self) -> &BlockStatus {
        &self.status
    }

    pub fn block_hash(&self) -> BlockHash {
        self.header.block_hash.0
    }

    pub fn parent_hash(&self) -> BlockHash {
        self.header.block_header_without_hash.parent_hash.0
    }

    pub fn sequencer_address(&self) -> ContractAddress {
        self.header.block_header_without_hash.sequencer.0.into()
    }

    pub fn timestamp(&self) -> BlockTimestamp {
        self.header.block_header_without_hash.timestamp
    }

    pub fn new_root(&self) -> Felt {
        self.header.block_header_without_hash.state_root.0
    }

    pub(crate) fn set_block_hash(&mut self, block_hash: BlockHash) {
        self.header.block_hash = starknet_api::block::BlockHash(block_hash);
    }

    pub fn block_number(&self) -> BlockNumber {
        self.header.block_header_without_hash.block_number
    }

    pub(crate) fn create_pending_block() -> Self {
        Self {
            header: BlockHeader {
                block_header_without_hash: BlockHeaderWithoutHash {
                    l1_da_mode: L1DataAvailabilityMode::Blob,
                    ..Default::default()
                },
                ..BlockHeader::default()
            },
            status: BlockStatus::Pending,
            transaction_hashes: Vec::new(),
        }
    }

    pub fn create_empty_accepted() -> Self {
        Self {
            header: BlockHeader::default(),
            transaction_hashes: vec![],
            status: BlockStatus::AcceptedOnL2,
        }
    }

    pub(crate) fn set_block_number(&mut self, block_number: u64) {
        self.header.block_header_without_hash.block_number = BlockNumber(block_number)
    }

    pub(crate) fn set_timestamp(&mut self, timestamp: BlockTimestamp) {
        self.header.block_header_without_hash.timestamp = timestamp;
    }
}

impl HashProducer for StarknetBlock {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<BlockHash> {
        let hash = Pedersen::hash_array(&[
            felt!(self.header.block_header_without_hash.block_number.0), // block number
            self.header.block_header_without_hash.state_root.0,          // global_state_root
            *self.header.block_header_without_hash.sequencer.0.key(),    // sequencer_address
            Felt::ZERO,                                                  /* block_timestamp;
                                                                          * would normally be
                                                                          * felt!(self.header.
                                                                          * timestamp.0), but
                                                                          * is modified to enable replicability
                                                                          * in re-execution on
                                                                          * loading on dump */
            felt!(self.transaction_hashes.len() as u64), // transaction_count
            Felt::ZERO,                                  // transaction_commitment
            Felt::ZERO,                                  // event_count
            Felt::ZERO,                                  // event_commitment
            Felt::ZERO,                                  // protocol_version
            Felt::ZERO,                                  // extra_data
            self.header.block_header_without_hash.parent_hash.0, // parent_block_hash
        ]);

        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{
        BlockHash, BlockHeader, BlockHeaderWithoutHash, BlockNumber, BlockStatus,
    };
    use starknet_api::data_availability::L1DataAvailabilityMode;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
    use starknet_types::traits::HashProducer;

    use super::{StarknetBlock, StarknetBlocks};
    use crate::state::state_diff::StateDiff;
    use crate::traits::HashIdentified;

    #[test]
    fn get_blocks_return_in_correct_order() {
        let mut blocks = StarknetBlocks::default();
        for block_number in 1..=10 {
            let mut block_to_insert = StarknetBlock::create_pending_block();
            block_to_insert.header.block_header_without_hash.block_number =
                BlockNumber(block_number);
            block_to_insert.header.block_hash =
                starknet_api::block::BlockHash(Felt::from(block_number as u128));
            blocks.insert(block_to_insert, StateDiff::default());
            blocks.pending_block.header.block_header_without_hash.block_number =
                BlockNumber(block_number).unchecked_next();
        }

        let block_numbers: Vec<u64> = blocks
            .get_blocks(None, None)
            .unwrap()
            .iter()
            .map(|block| block.block_number().0)
            .collect();
        assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], block_numbers);

        let block_numbers: Vec<u64> = blocks
            .get_blocks(Some(BlockId::Number(7)), None)
            .unwrap()
            .iter()
            .map(|block| block.block_number().0)
            .collect();
        assert_eq!(vec![7, 8, 9, 10, 11], block_numbers);

        let block_numbers: Vec<u64> = blocks
            .get_blocks(Some(BlockId::Number(7)), Some(BlockId::Tag(BlockTag::Latest)))
            .unwrap()
            .iter()
            .map(|block| block.block_number().0)
            .collect();
        assert_eq!(vec![7, 8, 9, 10], block_numbers);
    }

    #[test]
    fn block_number_from_block_id_should_return_correct_result() {
        let mut blocks = StarknetBlocks::new(0, None);
        let mut block_to_insert = StarknetBlock::create_pending_block();
        blocks.pending_block = block_to_insert.clone();

        // latest block returns none, because collection is empty
        assert!(
            blocks
                .block_number_from_block_id(&BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Latest
                ))
                .is_none()
        );
        // pending block returns some
        assert!(
            blocks
                .block_number_from_block_id(&BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Pending
                ))
                .is_some()
        );

        let block_hash = block_to_insert.generate_hash().unwrap();
        block_to_insert.header.block_header_without_hash.block_number = BlockNumber(10);
        block_to_insert.header.block_hash = starknet_api::block::BlockHash(block_hash);

        blocks.insert(block_to_insert, StateDiff::default());

        // returns block number, even if the block number is not present in the collection
        assert!(blocks.block_number_from_block_id(&BlockId::Number(11)).is_none());
        assert!(blocks.block_number_from_block_id(&BlockId::Number(10)).is_some());
        // returns none because there is no block with the given hash
        assert!(blocks.block_number_from_block_id(&BlockId::Hash(Felt::ONE)).is_none());
        assert!(
            blocks
                .block_number_from_block_id(&BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Latest
                ))
                .is_some()
        );
        assert!(
            blocks
                .block_number_from_block_id(&BlockId::Tag(
                    starknet_rs_core::types::BlockTag::Pending
                ))
                .is_some()
        );
        assert!(blocks.block_number_from_block_id(&BlockId::Hash(block_hash)).is_some());
    }

    #[test]
    fn get_blocks_with_filter() {
        let mut blocks = StarknetBlocks::default();

        let last_block_number = 11;
        for block_number in 2..=last_block_number {
            let mut block_to_insert = StarknetBlock::create_pending_block();
            block_to_insert.header.block_header_without_hash.block_number =
                BlockNumber(block_number);
            block_to_insert.header.block_hash =
                starknet_api::block::BlockHash(Felt::from(block_number as u128));
            blocks.insert(block_to_insert.clone(), StateDiff::default());

            // last block will be a pending block
            if block_number == last_block_number {
                blocks.pending_block = block_to_insert;
            }
        }

        // check blocks len
        assert!(blocks.hash_to_block.len() == 10);

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
        assert_eq!(blocks.get_blocks(Some(BlockId::Hash(Felt::from(9))), None).unwrap().len(), 3);
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
        assert_eq!(blocks.get_blocks(None, Some(BlockId::Hash(Felt::from(9)))).unwrap().len(), 8);
        // to filter using invalid hash
        assert!(blocks.get_blocks(None, Some(BlockId::Hash(Felt::ZERO))).is_err());
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
        // from block number to block hash
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Number(2)), Some(BlockId::Hash(Felt::from(9))))
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
                .get_blocks(Some(BlockId::Hash(Felt::TWO)), Some(BlockId::Hash(Felt::from(9))))
                .unwrap()
                .len(),
            8
        );
        assert!(
            blocks
                .get_blocks(Some(BlockId::Hash(Felt::TWO)), Some(BlockId::Hash(Felt::ZERO)))
                .is_err()
        );
        assert!(
            blocks
                .get_blocks(Some(BlockId::Hash(Felt::from(10))), Some(BlockId::Hash(Felt::from(5))))
                .unwrap()
                .is_empty()
        );
        // from block hash to block number
        assert_eq!(
            blocks
                .get_blocks(Some(BlockId::Hash(Felt::TWO)), Some(BlockId::Number(9)))
                .unwrap()
                .len(),
            8
        );
        // from last block hash to latest/pending
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(11))),
                    Some(BlockId::Tag(BlockTag::Latest))
                )
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            blocks
                .get_blocks(
                    Some(BlockId::Hash(Felt::from(11))),
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
                    Some(BlockId::Hash(Felt::from(11)))
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
                    Some(BlockId::Hash(Felt::from(11)))
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
                .get_blocks(Some(BlockId::Tag(BlockTag::Latest)), Some(BlockId::Hash(Felt::TWO)))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn get_by_block_id_is_correct() {
        let mut blocks = StarknetBlocks::default();
        let mut block_to_insert = StarknetBlock::create_pending_block();
        block_to_insert.header.block_hash =
            starknet_api::block::BlockHash(block_to_insert.generate_hash().unwrap());
        block_to_insert.header.block_header_without_hash.block_number = BlockNumber(10);
        blocks.pending_block = block_to_insert.clone();

        blocks.insert(block_to_insert.clone(), StateDiff::default());

        let extracted_block = blocks.get_by_block_id(&BlockId::Number(10)).unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block =
            blocks.get_by_block_id(&BlockId::Hash(block_to_insert.block_hash())).unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block = blocks
            .get_by_block_id(&BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
            .unwrap();
        assert!(block_to_insert == extracted_block.clone());

        let extracted_block = blocks
            .get_by_block_id(&BlockId::Tag(starknet_rs_core::types::BlockTag::Pending))
            .unwrap();
        assert!(block_to_insert == extracted_block.clone());

        match blocks.get_by_block_id(&BlockId::Number(11)) {
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
            block.header.block_header_without_hash.block_number = BlockNumber(block_number);
            block.set_block_hash(block.generate_hash().unwrap());

            blocks.insert(block, StateDiff::default());
        }

        assert!(
            blocks
                .get_by_num(&BlockNumber(0))
                .unwrap()
                .header
                .block_header_without_hash
                .parent_hash
                == BlockHash::default()
        );
        assert!(
            blocks.get_by_num(&BlockNumber(0)).unwrap().header.block_hash
                == blocks
                    .get_by_num(&BlockNumber(1))
                    .unwrap()
                    .header
                    .block_header_without_hash
                    .parent_hash
        );
        assert!(
            blocks.get_by_num(&BlockNumber(1)).unwrap().header.block_hash
                == blocks
                    .get_by_num(&BlockNumber(2))
                    .unwrap()
                    .header
                    .block_header_without_hash
                    .parent_hash
        );
        assert!(
            blocks
                .get_by_num(&BlockNumber(1))
                .unwrap()
                .header
                .block_header_without_hash
                .parent_hash
                != blocks
                    .get_by_num(&BlockNumber(2))
                    .unwrap()
                    .header
                    .block_header_without_hash
                    .parent_hash
        )
    }

    #[test]
    fn get_by_hash_is_correct() {
        let mut blocks = StarknetBlocks::default();
        let mut block_to_insert = StarknetBlock::create_pending_block();
        block_to_insert.header.block_hash =
            starknet_api::block::BlockHash(block_to_insert.generate_hash().unwrap());
        block_to_insert.header.block_header_without_hash.block_number = BlockNumber(1);

        blocks.insert(block_to_insert.clone(), StateDiff::default());

        let extracted_block = blocks.get_by_hash(block_to_insert.block_hash()).unwrap();
        assert!(block_to_insert == extracted_block.clone());
    }

    #[test]
    fn check_pending_block() {
        let block = StarknetBlock::create_pending_block();
        assert!(block.status == BlockStatus::Pending);
        assert!(block.transaction_hashes.is_empty());
        assert_eq!(
            block.header,
            BlockHeader {
                block_header_without_hash: BlockHeaderWithoutHash {
                    l1_da_mode: L1DataAvailabilityMode::Blob,
                    ..Default::default()
                },
                ..Default::default()
            }
        );
    }
}
