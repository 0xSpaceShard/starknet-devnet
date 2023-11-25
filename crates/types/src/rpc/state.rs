use serde::{Deserialize, Serialize};

use super::block::GlobalRootHex;
use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, ClassHash, Felt, Nonce};
use crate::patricia_key::PatriciaKey;

pub type CompiledClassHashHex = Felt;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateUpdate {
    pub block_hash: BlockHash,
    pub new_root: GlobalRootHex,
    pub old_root: GlobalRootHex,
    pub state_diff: ThinStateDiff,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ThinStateDiff {
    pub deployed_contracts: Vec<DeployedContract>,
    pub storage_diffs: Vec<StorageDiff>,
    pub declared_classes: Vec<ClassHashes>,
    pub deprecated_declared_classes: Vec<ClassHash>,
    pub nonces: Vec<ContractNonce>,
    pub replaced_classes: Vec<ReplacedClasses>,
}

/// A deployed contract in Starknet.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployedContract {
    pub address: ContractAddress,
    pub class_hash: ClassHash,
}

/// Storage differences in Starknet.
// Invariant: Storage keys are strictly increasing. In particular, no key appears twice.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct StorageDiff {
    pub address: ContractAddress,
    pub storage_entries: Vec<StorageEntry>,
}

/// A storage entry in a contract.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct StorageEntry {
    pub key: PatriciaKey,
    pub value: Felt,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ClassHashes {
    pub class_hash: ClassHash,
    pub compiled_class_hash: CompiledClassHashHex,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReplacedClasses {
    pub contract_address: ContractAddress,
    pub class_hash: ClassHash,
}

/// The nonce of a Starknet contract.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractNonce {
    pub contract_address: ContractAddress,
    pub nonce: Nonce,
}
