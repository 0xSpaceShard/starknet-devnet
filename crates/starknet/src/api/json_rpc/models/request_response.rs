use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::block::BlockNumber;

use super::{
    block::{BlockHashHex, SyncStatus},
    transaction::{ClassHashHex, EventFilter, FunctionCall, Transaction, TransactionHashHex},
    BlockId, ContractAddressHex, PatriciaKeyHex,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockIdInput {
    pub(crate) block_id: BlockId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TransactionHashInput {
    pub(crate) transaction_hash: TransactionHashHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct GetStorageInput {
    pub(crate) block_id: BlockId,
    pub(crate) contract_address: ContractAddressHex,
    pub(crate) key: PatriciaKeyHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndIndexInput {
    pub(crate) block_id: BlockId,
    pub(crate) index: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndClassHashInput {
    pub(crate) block_id: BlockId,
    pub(crate) class_hash: ClassHashHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndContractAddressInput {
    pub(crate) block_id: BlockId,
    pub(crate) contract_address: ContractAddressHex,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct CallInput {
    pub request: FunctionCall,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EstimateFeeInput {
    pub request: Vec<Transaction>,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EstimateFeeOutput {
    pub gas_consumed: String,
    pub gas_price: String,
    pub overall_fee: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockHashAndNumberOutput {
    pub block_hash: BlockHashHex,
    pub block_number: BlockNumber,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum SyncingOutput {
    True(SyncStatus),
    False(bool),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventsInput {
    pub filter: EventFilter,
}
