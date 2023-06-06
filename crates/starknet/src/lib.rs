use block::StarknetBlocks;
use state::StarknetState;
use transaction::StarknetTransactions;

mod account;
mod block;
mod constants;
mod predeployed_account;
mod state;
mod traits;
mod transaction;
mod utils;

pub(crate) struct Starknet {
    pub blocks: StarknetBlocks,
    //pub block_context: BlockContext,
    pub state: StarknetState,
    pub transactions: StarknetTransactions,
}
