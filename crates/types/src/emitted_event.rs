use serde::Serialize;
use starknet_api::block::BlockNumber;

use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, Felt, TransactionHash};

#[derive(Serialize, Clone, Debug)]
pub struct EmittedEvent {
    pub transaction_hash: TransactionHash,
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    #[serde(flatten)]
    pub event_data: Event,
}

#[derive(Serialize, Clone, Debug)]
pub struct Event {
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}
