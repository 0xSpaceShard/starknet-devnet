use num_bigint::BigUint;
use serde::Serialize;
use starknet_types_core::felt::Felt;

use super::block::BlockRoot;
use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, ClassHash, Nonce};
use crate::patricia_key::PatriciaKey;

pub type CompiledClassHashHex = Felt;
pub type Balance = BigUint;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub enum StateUpdateResult {
    StateUpdate(StateUpdate),
    PendingStateUpdate(PendingStateUpdate),
}

impl StateUpdateResult {
    pub fn get_state_diff(&self) -> &ThinStateDiff {
        match self {
            StateUpdateResult::StateUpdate(s) => &s.state_diff,
            StateUpdateResult::PendingStateUpdate(s) => &s.state_diff,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct StateUpdate {
    pub block_hash: BlockHash,
    pub new_root: BlockRoot,
    pub old_root: BlockRoot,
    pub state_diff: ThinStateDiff,
}

impl StateUpdate {
    /// New and old root are not computed - Devnet does not store block data in a tree.
    pub fn new(block_hash: Felt, state_diff: ThinStateDiff) -> Self {
        Self { block_hash, new_root: Felt::default(), old_root: Felt::default(), state_diff }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct PendingStateUpdate {
    pub old_root: BlockRoot,
    pub state_diff: ThinStateDiff,
}

#[derive(Debug, Default, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, Eq, PartialEq),
    serde(deny_unknown_fields)
)]
pub struct ThinStateDiff {
    pub deployed_contracts: Vec<DeployedContract>,
    pub storage_diffs: Vec<StorageDiff>,
    pub declared_classes: Vec<ClassHashPair>,
    pub deprecated_declared_classes: Vec<ClassHash>,
    pub nonces: Vec<ContractNonce>,
    pub replaced_classes: Vec<ReplacedClasses>,
}

/// A deployed contract in Starknet.
#[derive(Debug, Default, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, Eq, PartialEq),
    serde(deny_unknown_fields)
)]
pub struct DeployedContract {
    pub address: ContractAddress,
    pub class_hash: ClassHash,
}

/// Storage differences in Starknet.
// Invariant: Storage keys are strictly increasing. In particular, no key appears twice.
#[derive(Debug, Default, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, Eq, PartialEq),
    serde(deny_unknown_fields)
)]
pub struct StorageDiff {
    pub address: ContractAddress,
    pub storage_entries: Vec<StorageEntry>,
}

/// A storage entry in a contract.
#[derive(Debug, Default, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, Eq, PartialEq),
    serde(deny_unknown_fields)
)]
pub struct StorageEntry {
    pub key: PatriciaKey,
    pub value: Felt,
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq, serde::Deserialize))]
pub struct ClassHashPair {
    pub class_hash: ClassHash,
    pub compiled_class_hash: CompiledClassHashHex,
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "testing", derive(Eq, PartialEq, serde::Deserialize))]
pub struct ReplacedClasses {
    pub contract_address: ContractAddress,
    pub class_hash: ClassHash,
}

/// The nonce of a Starknet contract.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, Eq, PartialEq),
    serde(deny_unknown_fields)
)]
pub struct ContractNonce {
    pub contract_address: ContractAddress,
    pub nonce: Nonce,
}
