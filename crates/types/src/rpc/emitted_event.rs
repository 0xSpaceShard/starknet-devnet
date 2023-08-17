use serde::Serialize;
use starknet_api::block::BlockNumber;

use crate::contract_address::ContractAddress;
use crate::rpc::felt::{BlockHash, Felt, TransactionHash};

#[derive(Serialize, Clone, Debug)]
pub struct EmittedEvent {
    pub transaction_hash: TransactionHash,
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}
