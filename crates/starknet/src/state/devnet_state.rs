use std::collections::HashMap;

use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, CompiledClassHash, Felt};

use crate::error::{DevnetResult, Error, StateError};

#[derive(Default, Clone)]
pub(crate) struct DevnetState {
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub address_to_nonce: HashMap<ContractAddress, Felt>,
    pub address_to_storage: HashMap<ContractStorageKey, Felt>,
    pub class_hash_to_compiled_class: HashMap<ClassHash, ContractClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
}

impl crate::traits::DevnetStateReader for DevnetState {
    fn compiled_class_hash_at(&self, class_hash: &ClassHash) -> DevnetResult<ClassHash> {
        Ok(self.class_hash_to_compiled_class_hash.get(class_hash).cloned().unwrap_or_default())
    }

    fn storage_at(&self, storage_key: &ContractStorageKey) -> DevnetResult<Felt> {
        Ok(self.address_to_storage.get(storage_key).cloned().unwrap_or_default())
    }

    fn nonce_at(&self, address: &ContractAddress) -> DevnetResult<Felt> {
        Ok(self.address_to_nonce.get(address).cloned().unwrap_or_default())
    }

    fn class_hash_at(&self, address: &ContractAddress) -> DevnetResult<ClassHash> {
        Ok(self.address_to_class_hash.get(address).cloned().unwrap_or_default())
    }

    fn contract_class_at(&self, class_hash: &ClassHash) -> DevnetResult<ContractClass> {
        if let Some(deprecated_contract_class) = self.class_hash_to_compiled_class.get(class_hash) {
            Ok(deprecated_contract_class.clone())
        } else {
            let compiled_class_hash = self
                .class_hash_to_compiled_class_hash
                .get(class_hash)
                .ok_or(Error::StateError(StateError::NoneCompiledHash(*class_hash)))?;

            self.class_hash_to_compiled_class
                .get(compiled_class_hash)
                .ok_or(Error::StateError(StateError::NoneCasmClass(*compiled_class_hash)))
                .cloned()
        }
    }
}
