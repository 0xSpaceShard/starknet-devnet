#[cfg(test)]
pub(crate) mod test_utils {
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;
    use starknet_types::patricia_key::StorageKey;

    use crate::constants;

    pub(crate) const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
        "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

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

    pub(crate) fn dummy_contract_address() -> ContractAddress {
        ContractAddress::new(Felt::from_prefixed_hex_str("0xADD4E55").unwrap()).unwrap()
    }

    pub(crate) fn get_bytes_from_u32(num: u32) -> [u8; 32] {
        let num_bytes = num.to_be_bytes();
        let mut result = [0u8; 32];
        let starting_idx = result.len() - num_bytes.len();
        let ending_idx = result.len();

        result[starting_idx..ending_idx].copy_from_slice(&num_bytes[..(ending_idx - starting_idx)]);

        result
    }
}
