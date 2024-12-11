use starknet_rs_core::types::Felt;
use starknet_types::rpc::state::ThinStateDiff;

pub struct StateUpdate {
    pub block_hash: Felt,
    pub new_root: Felt,
    pub old_root: Felt,
    pub state_diff: ThinStateDiff,
}

impl StateUpdate {
    pub fn new(block_hash: Felt, state_diff: ThinStateDiff) -> Self {
        // TODO new and old root are not computed, they are not part of the MVP
        Self { block_hash, new_root: Felt::default(), old_root: Felt::default(), state_diff }
    }
}
