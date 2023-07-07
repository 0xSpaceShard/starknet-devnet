use serde::{Deserialize, Serialize};

use super::block::{BlockHashHex, GlobalRootHex};
use super::transaction::{ClassHashHex, Nonce};
use super::{ContractAddressHex, FeltHex, PatriciaKeyHex};

pub type CompiledClassHashHex = FeltHex;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct StateUpdate {
    pub block_hash: BlockHashHex,
    pub new_root: GlobalRootHex,
    pub old_root: GlobalRootHex,
    pub state_diff: ThinStateDiff,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ThinStateDiff {
    pub deployed_contracts: Vec<DeployedContract>,
    pub storage_diffs: Vec<StorageDiff>,
    pub declared_classes: Vec<ClassHashes>,
    pub deprecated_declared_classes: Vec<ClassHashHex>,
    pub nonces: Vec<ContractNonce>,
    pub replaced_classes: Vec<ReplacedClasses>,
}

/// A deployed contract in StarkNet.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployedContract {
    pub address: ContractAddressHex,
    pub class_hash: ClassHashHex,
}

/// Storage differences in StarkNet.
// Invariant: Storage keys are strictly increasing. In particular, no key appears twice.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct StorageDiff {
    pub address: ContractAddressHex,
    storage_entries: Vec<StorageEntry>,
}

/// A storage entry in a contract.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct StorageEntry {
    pub key: PatriciaKeyHex,
    pub value: FeltHex,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ClassHashes {
    pub class_hash: ClassHashHex,
    pub compiled_class_hash: CompiledClassHashHex,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReplacedClasses {
    pub contract_address: ContractAddressHex,
    pub class_hash: ClassHashHex,
}

/// The nonce of a StarkNet contract.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractNonce {
    pub contract_address: ContractAddressHex,
    pub nonce: Nonce,
}
