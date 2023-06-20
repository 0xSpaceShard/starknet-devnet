use serde::{Deserialize, Serialize};

use super::{transaction::{TransactionHashHex, ClassHashHex}, BlockId, ContractAddressHex, PatriciaKeyHex};

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
    pub(crate) index: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndClassHashInput {
    pub(crate) block_id: BlockId,
    pub(crate) class_hash: ClassHashHex
}