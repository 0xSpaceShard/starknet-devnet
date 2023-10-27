use std::fmt::Display;

use super::contract_address::ContractAddress;
use crate::patricia_key::StorageKey;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractStorageKey(ContractAddress, StorageKey);

impl ContractStorageKey {
    pub fn new(address: ContractAddress, storage_key: StorageKey) -> Self {
        Self(address, storage_key)
    }

    pub fn get_contract_address(&self) -> &ContractAddress {
        &self.0
    }

    pub fn get_storage_key(&self) -> &StorageKey {
        &self.1
    }
}

impl Display for ContractStorageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("({0:x}, {1:x})", self.0, self.1.0))
    }
}
