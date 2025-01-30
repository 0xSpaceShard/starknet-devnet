use std::collections::HashMap;

use blockifier::execution::contract_class::RunnableCompiledClass;
use blockifier::state::cached_state::StorageEntry;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::{StateReader, StateResult};
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::state::StorageKey;
use starknet_rs_core::types::Felt;

use crate::starknet::defaulter::StarknetDefaulter;

/// A simple implementation of `StateReader` using `HashMap`s as storage.
/// Copied from blockifier test_utils, added `impl State`
#[derive(Debug, Default, Clone)]
pub struct DictState {
    pub storage_view: HashMap<StorageEntry, Felt>,
    pub address_to_nonce: HashMap<ContractAddress, Nonce>,
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub class_hash_to_class: HashMap<ClassHash, RunnableCompiledClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
    defaulter: StarknetDefaulter,
}

impl DictState {
    pub fn new(defaulter: StarknetDefaulter) -> Self {
        Self { defaulter, ..Self::default() }
    }
}

impl StateReader for DictState {
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<Felt> {
        let contract_storage_key = (contract_address, key);
        match self.storage_view.get(&contract_storage_key) {
            Some(value) => Ok(*value),
            None => self.defaulter.get_storage_at(contract_address, key),
        }
    }

    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        match self.address_to_nonce.get(&contract_address) {
            Some(value) => Ok(*value),
            None => self.defaulter.get_nonce_at(contract_address),
        }
    }

    fn get_compiled_class(&self, class_hash: ClassHash) -> StateResult<RunnableCompiledClass> {
        match self.class_hash_to_class.get(&class_hash) {
            Some(contract_class) => Ok(contract_class.clone()),
            None => self.defaulter.get_compiled_class(class_hash),
        }
    }

    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        match self.address_to_class_hash.get(&contract_address) {
            Some(class_hash) => Ok(*class_hash),
            None => self.defaulter.get_class_hash_at(contract_address),
        }
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: ClassHash,
    ) -> StateResult<starknet_api::core::CompiledClassHash> {
        // can't ask origin for this - insufficient API - probably not important
        let compiled_class_hash =
            self.class_hash_to_compiled_class_hash.get(&class_hash).copied().unwrap_or_default();
        Ok(compiled_class_hash)
    }
}

// Basing the methods on blockifier's `State` interface, without those that would never be used
// (add_visited_pcs, to_state_diff)
impl DictState {
    pub fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
        value: Felt,
    ) -> std::result::Result<(), blockifier::state::errors::StateError> {
        self.storage_view.insert((contract_address, key), value);
        Ok(())
    }

    pub fn increment_nonce(&mut self, contract_address: ContractAddress) -> StateResult<()> {
        let current_nonce = self.get_nonce_at(contract_address)?;
        let next_nonce = Nonce(current_nonce.0 + 1);
        self.address_to_nonce.insert(contract_address, next_nonce);

        Ok(())
    }

    pub fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()> {
        if contract_address == ContractAddress::default() {
            return Err(StateError::OutOfRangeContractAddress);
        }

        self.address_to_class_hash.insert(contract_address, class_hash);
        Ok(())
    }

    pub fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: RunnableCompiledClass,
    ) -> StateResult<()> {
        self.class_hash_to_class.insert(class_hash, contract_class);
        Ok(())
    }

    pub fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        self.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
        Ok(())
    }
}
