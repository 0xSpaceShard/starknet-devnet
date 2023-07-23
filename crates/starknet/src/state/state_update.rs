use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, Felt};

use super::state_diff::StateDiff;
use crate::error::Result;

pub struct StateUpdate {
    pub block_hash: Felt,
    pub new_root: Felt,
    pub old_root: Felt,
    pub declared_classes: Vec<(ClassHash, ClassHash)>,
    pub cairo_0_declared_classes: Vec<Felt>,
    pub storage_updates: Vec<(ContractAddress, Vec<(Felt, Felt)>)>,
    pub nonces: Vec<(ContractAddress, Felt)>,
}

impl StateUpdate {
    pub fn new(block_hash: Felt, state_diff: StateDiff) -> Result<Self> {
        let declared_classes = state_diff
            .class_hash_to_compiled_class_hash
            .into_iter()
            .map(|(class_hash, compiled_class_hash)| (class_hash, class_hash))
            .collect();

        let cairo_0_declared_classes: Vec<Felt> = state_diff
            .cairo_0_declared_contracts
            .into_iter()
            .map(|(class_hash, _)| class_hash)
            .collect();

        let mut storage_updates = Vec::<(ContractAddress, Vec<(Felt, Felt)>)>::new();

        for (address, storage_entry) in state_diff.inner.storage_updates() {
            let storage_entry_updates = storage_entry
                .clone()
                .into_iter()
                .map(|(key, value)| (Felt::from(key), Felt::from(value)))
                .collect();

            let contract_address =
                ContractAddress::try_from(address.clone()).map_err(crate::error::Error::from)?;

            storage_updates.push((contract_address, storage_entry_updates));
        }

        let mut nonces = Vec::<(ContractAddress, Felt)>::new();
        for (address, nonce) in state_diff.inner.address_to_nonce() {
            let contract_address =
                ContractAddress::try_from(address.clone()).map_err(crate::error::Error::from)?;

            nonces.push((contract_address, Felt::from(nonce.clone())));
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
        })
    }
}
