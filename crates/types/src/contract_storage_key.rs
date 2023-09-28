use starknet_in_rust::utils::Address;

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
}

impl From<&ContractStorageKey> for starknet_in_rust::state::state_cache::StorageEntry {
    fn from(value: &ContractStorageKey) -> Self {
        (Address::from(&value.0), value.1.0.bytes())
    }
}

impl From<ContractStorageKey> for starknet_in_rust::state::state_cache::StorageEntry {
    fn from(value: ContractStorageKey) -> Self {
        (Address::from(value.0), value.1.0.bytes())
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::state::state_cache::StorageEntry as StarknetInRustStorageEntry;

    use super::ContractStorageKey;
    use crate::contract_address::{test_utils, ContractAddress};
    use crate::felt::Felt;
    use crate::patricia_key::PatriciaKey;
    use crate::utils::test_utils::dummy_felt;

    #[test]
    fn correct_convertion_to_starknet_in_rust_storage_entry() {
        let address = ContractAddress::new(dummy_felt()).unwrap();

        let storage_key = ContractStorageKey::new(
            address,
            PatriciaKey::new(Felt::from_prefixed_hex_str("0xFF").unwrap()).unwrap(),
        );

        let storage_entry: StarknetInRustStorageEntry = TryFrom::try_from(&storage_key).unwrap();

        assert!(test_utils::is_equal(&storage_key.0, &storage_entry.0));
        assert_eq!(storage_key.1.0.bytes(), storage_entry.1);
    }
}
