use starknet_in_rust::utils::Address;

use super::contract_address::ContractAddress;
use crate::error::Error;
use crate::patricia_key::StorageKey;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ContractStorageKey(ContractAddress, StorageKey);

impl ContractStorageKey {
    pub fn new(address: ContractAddress, storage_key: StorageKey) -> Self {
        Self(address, storage_key)
    }
}

impl TryFrom<&ContractStorageKey>
    for starknet_in_rust::business_logic::state::state_cache::StorageEntry
{
    type Error = Error;
    fn try_from(value: &ContractStorageKey) -> Result<Self, Self::Error> {
        Ok((Address::try_from(&value.0)?, value.1.0.bytes()))
    }
}

impl TryFrom<ContractStorageKey>
    for starknet_in_rust::business_logic::state::state_cache::StorageEntry
{
    type Error = Error;
    fn try_from(value: ContractStorageKey) -> Result<Self, Self::Error> {
        Ok((Address::try_from(value.0)?, value.1.0.bytes()))
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::business_logic::state::state_cache::StorageEntry as StarknetInRustStorageEntry;

    use super::ContractStorageKey;
    use crate::contract_address::{test_utils, ContractAddress};
    use crate::felt::Felt;
    use crate::patricia_key::PatriciaKey;
    use crate::test_utils::dummy_felt;

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
