use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::patricia_key::PatriciaKey;

use super::state_diff::StateDiff;
use crate::error::DevnetResult;

pub struct StateUpdate {
    pub block_hash: Felt,
    pub new_root: Felt,
    pub old_root: Felt,
    pub declared_classes: Vec<(ClassHash, ClassHash)>,
    pub cairo_0_declared_classes: Vec<Felt>,
    pub storage_updates: Vec<(ContractAddress, Vec<(PatriciaKey, Felt)>)>,
    pub nonces: Vec<(ContractAddress, Felt)>,
    pub deployed_contracts: Vec<(ContractAddress, ClassHash)>,
}

impl StateUpdate {
    pub fn new(block_hash: Felt, state_diff: StateDiff) -> DevnetResult<Self> {
        // declared classes (class hash, compiled class hash) that are not cairo 0
        let declared_classes = state_diff.class_hash_to_compiled_class_hash.into_iter().collect();

        // cairo 0 declarations
        let cairo_0_declared_classes: Vec<Felt> = state_diff.cairo_0_declared_contracts;

        // storage updates (contract address -> [(storage_entry, value)])
        let storage_updates = state_diff
            .storage_updates
            .into_iter()
            .map(|(address, entries)| (address, entries.into_iter().collect()))
            .collect();

        // contract nonces
        let nonces = state_diff.address_to_nonce.into_iter().collect();

        // deployed contracts (address -> class hash)
        let deployed_contracts = state_diff.address_to_class_hash.into_iter().collect();

        // TODO new and old root are not computed, they are not part of the MVP
        Ok(Self {
            block_hash,
            new_root: Felt::default(),
            old_root: Felt::default(),
            declared_classes,
            cairo_0_declared_classes,
            storage_updates,
            nonces,
            deployed_contracts,
        })
    }
}
