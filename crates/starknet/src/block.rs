use std::collections::HashMap;

use starknet_api::{
    block::{Block, BlockBody, BlockHash, BlockHeader, BlockNumber, BlockStatus, BlockTimestamp, GasPrice},
    core::{ContractAddress, GlobalRoot},
    transaction::{Transaction, TransactionOutput},
};

#[derive(Debug, Clone)]
pub(crate) struct StarknetBlock {
    inner: Block,
    status: Option<BlockStatus>,
}

impl StarknetBlock {
    pub fn new(
        block_hash: BlockHash,
        parent_hash: BlockHash,
        block_number: BlockNumber,
        gas_price: GasPrice,
        state_root: GlobalRoot,
        sequencer: ContractAddress,
        timestamp: BlockTimestamp,
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        status: Option<BlockStatus>,
    ) -> Self {
        Self {
            inner: Block {
                header: BlockHeader {
                    block_hash,
                    parent_hash,
                    block_number,
                    gas_price,
                    state_root,
                    sequencer,
                    timestamp,
                },
                body: BlockBody { transactions, transaction_outputs },
            },
            status,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct StarknetBlocks {
    hash_to_num: HashMap<BlockHash, BlockNumber>,
    num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pending_block: Option<StarknetBlock>,
}
