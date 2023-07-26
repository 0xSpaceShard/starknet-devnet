use starknet_rs_core::types::BlockId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;

use super::Starknet;

pub(crate) fn get_events(
    starknet: &Starknet,
    from_block: Option<BlockId>,
    to_block: Option<BlockId>,
    address: ContractAddress,
    keys: Vec<Felt>,
) {
    // starknet.blocks.
}
