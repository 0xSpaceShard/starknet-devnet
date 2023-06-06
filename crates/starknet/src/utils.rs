use std::fs;

use serde::Deserialize;
use starknet_in_rust::core::contract_address::starknet_contract_address::compute_deprecated_class_hash;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass;
use starknet_rs_core::types::contract::legacy::LegacyContractClass;

use starknet_types::error::{Error, JsonError};
use starknet_types::{
    felt::{ClassHash, Felt},
    DevnetResult,
};

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

pub(crate) fn generate_u128_random_numbers(seed: u32, random_numbers_count: u8) -> DevnetResult<Vec<u128>> {
    Ok(random_number_generator::generate_u128_random_numbers(seed, random_numbers_count))
}

pub(crate) fn load_cairo_0_contract_class<T>(path: &str) -> DevnetResult<T>
where
    T: for<'a> Deserialize<'a>,
{
    let contract_class_str = fs::read_to_string(path).map_err(Error::IOError)?;
    let contract_class = serde_json::from_str(&contract_class_str).map_err(JsonError::SerdeJsonError)?;

    Ok(contract_class)
}

pub(crate) fn compute_cairo_0_class_hash(contract_class: &ContractClass) -> DevnetResult<ClassHash> {
    let class_hash_felt_252 =
        compute_deprecated_class_hash(contract_class).map_err(Error::StarknetInRustContractAddressError)?;

    Ok(Felt::new(class_hash_felt_252.to_be_bytes()))
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;

    use super::{generate_u128_random_numbers, get_bytes_from_u32, load_cairo_0_contract_class};
    use crate::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};
    use starknet_types::felt::Felt;

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
}
