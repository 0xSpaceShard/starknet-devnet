use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;

use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, Felt, TransactionHash};

#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmittedEvent {
    pub transaction_hash: TransactionHash,
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedEvent {
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
    pub order: usize,
}

impl From<&blockifier::execution::call_info::OrderedEvent> for OrderedEvent {
    fn from(event: &blockifier::execution::call_info::OrderedEvent) -> Self {
        Self {
            order: event.order,
            keys: event.event.keys.iter().map(|k| k.0.into()).collect(),
            data: event.event.data.0.iter().map(|d| (*d).into()).collect(),
        }
    }
}
