use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::Felt;
use starknet_types::patricia_key::StorageKey;

use crate::account::Account;
use crate::constants;

pub(crate) fn dummy_felt() -> Felt {
    Felt::from_prefixed_hex_str("0xDD10").unwrap()
}

pub(crate) fn dummy_contract_storage_key() -> ContractStorageKey {
    ContractStorageKey::new(
        ContractAddress::new(Felt::from_prefixed_hex_str("0xFE").unwrap()).unwrap(),
        StorageKey::try_from(dummy_felt()).unwrap(),
    )
}

pub(crate) fn dummy_contract_class() -> ContractClass {
    let json_str = std::fs::read_to_string(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();

    ContractClass::from_json_str(&json_str).unwrap()
}

pub(crate) fn dummy_contract_class_json_str() -> String {
    std::fs::read_to_string(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap()
}

pub(crate) fn dummy_contract_address() -> ContractAddress {
    ContractAddress::new(Felt::from_prefixed_hex_str("0xADD4E55").unwrap()).unwrap()
}
