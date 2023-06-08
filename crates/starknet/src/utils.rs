use std::fs;

use serde::Deserialize;
use starknet_api::hash::{pedersen_hash, StarkFelt};
use starknet_in_rust::utils::calculate_sn_keccak;
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_class::ContractClass;
use starknet_types::error::{Error, JsonError};
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::num_integer::Integer;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};
use starknet_types::DevnetResult;

const PY_RANDOM_NUMBER_GENERATOR_SCRIPT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/random_number_generator.py"));

pub(crate) fn get_bytes_from_u32(num: u32) -> [u8; 32] {
    let num_bytes = num.to_be_bytes();
    let mut result = [0u8; 32];
    let starting_idx = result.len() - num_bytes.len();
    let ending_idx = result.len();

    for idx in starting_idx..ending_idx {
        result[idx] = num_bytes[idx - starting_idx];
    }

    result
}

pub(crate) fn generate_u128_random_numbers(
    seed: u32,
    random_numbers_count: u8,
) -> DevnetResult<Vec<u128>> {
    Ok(random_number_generator::generate_u128_random_numbers(seed, random_numbers_count))
}

pub(crate) fn load_cairo_0_contract_class(path: &str) -> DevnetResult<ContractClass> {
    let contract_class_str = fs::read_to_string(path).map_err(Error::IOError)?;
    Ok(ContractClass::from_json_str(&contract_class_str)?)
}

/// Returns the storage address of a StarkNet storage variable given its name and arguments.
pub(crate) fn get_storage_var_address(
    storage_var_name: &str,
    args: &[Felt],
) -> DevnetResult<StorageKey> {
    let storage_var_name_hash = calculate_sn_keccak(storage_var_name.as_bytes());
    let storage_var_name_hash = StarkFelt::new(storage_var_name_hash)?;

    let storage_key_hash = args
        .iter()
        .fold(storage_var_name_hash, |res, arg| pedersen_hash(&res, &StarkFelt::from(arg)));

    let storage_key = Felt252::from_bytes_be(storage_key_hash.bytes()).mod_floor(
        &Felt252::from_bytes_be(&starknet_api::core::L2_ADDRESS_UPPER_BOUND.to_bytes_be()),
    );

    PatriciaKey::new(Felt::new(storage_key.to_be_bytes())?)
}

#[cfg(test)]
mod tests {

    use starknet_api::hash::StarkFelt;
    use starknet_types::traits::ToHexString;

    use super::{generate_u128_random_numbers, get_bytes_from_u32, get_storage_var_address};
    use crate::test_utils;

    #[test]
    fn correct_bytes_from_number() {
        let result = get_bytes_from_u32(123);
        assert!(result[31] == 123)
    }

    #[test]
    fn correct_number_generated_based_on_fixed_seed() {
        let generated_numbers = generate_u128_random_numbers(123, 2).unwrap();
        let expected_output: Vec<u128> =
            vec![261662301160200998434711212977610535782, 285327960644938307249498422906269531911];
        assert_eq!(generated_numbers, expected_output);
    }

    #[test]
    fn correct_simple_storage_var_address_generated() {
        let expected_storage_var_address =
            blockifier::abi::abi_utils::get_storage_var_address("simple", &[]).unwrap();
        let generated_storage_var_address = get_storage_var_address("simple", &[]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().bytes(),
            generated_storage_var_address.to_felt().bytes()
        );
    }

    #[test]
    fn correct_complex_storage_var_address_generated() {
        let prefixed_hex_felt_string = test_utils::dummy_felt().to_prefixed_hex_str();

        let expected_storage_var_address = blockifier::abi::abi_utils::get_storage_var_address(
            "complex",
            &[prefixed_hex_felt_string.as_str().try_into().unwrap()],
        )
        .unwrap();

        let generated_storage_var_address =
            get_storage_var_address("complex", &[test_utils::dummy_felt()]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().bytes(),
            generated_storage_var_address.to_felt().bytes()
        );
    }
}
