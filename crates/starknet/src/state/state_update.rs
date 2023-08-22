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
        let declared_classes = state_diff
            .class_hash_to_compiled_class_hash
            .into_iter()
            .map(|(class_hash, compiled_class_hash)| (class_hash, compiled_class_hash))
            .collect();

        // cairo 0 declarations
        let cairo_0_declared_classes: Vec<Felt> =
            state_diff.cairo_0_declared_contracts.into_keys().collect();

        // storage updates (contract address -> [(storage_entry, value)])
        let mut storage_updates = Vec::<(ContractAddress, Vec<(PatriciaKey, Felt)>)>::new();

        for (address, storage_entry) in state_diff.inner.storage_updates() {
            let mut storage_entry_updates = Vec::<(PatriciaKey, Felt)>::new();

            for (key, value) in storage_entry.clone().into_iter() {
                storage_entry_updates.push((PatriciaKey::new(Felt::from(key))?, Felt::from(value)));
            }

            let contract_address =
                ContractAddress::try_from(address.clone()).map_err(crate::error::Error::from)?;

            storage_updates.push((contract_address, storage_entry_updates));
        }

        // contract nonces
        let mut nonces = Vec::<(ContractAddress, Felt)>::new();
        for (address, nonce) in state_diff.inner.address_to_nonce() {
            let contract_address =
                ContractAddress::try_from(address.clone()).map_err(crate::error::Error::from)?;

            nonces.push((contract_address, Felt::from(nonce.clone())));
        }

        // deployed contracts (address -> class hash)
        let mut deployed_contracts = Vec::new();
        for (address, class_hash) in state_diff.inner.address_to_class_hash() {
            let contract_address =
                ContractAddress::try_from(address.clone()).map_err(crate::error::Error::from)?;
            let class_hash = Felt::new(*class_hash).map_err(crate::error::Error::from)?;
            deployed_contracts.push((contract_address, class_hash));
        }

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
