use std::fs;

use pyo3::{types::PyModule, Py, PyAny, PyResult, Python};
use starknet_in_rust::core::contract_address::starknet_contract_address::compute_deprecated_class_hash;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass;
use starknet_rs_core::types::contract::legacy::LegacyContractClass;

use crate::error::{Error, JsonError};
use crate::types::{ClassHash, DevnetResult, Felt};

const PY_RANDOM_NUMBER_GENERATOR_SCRIPT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/random_number_generator.py"));

const CAIRO_0_ACCOUNT_CONTRACT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/OpenZeppelin/0.5.1/Account.cairo/Account.json");

const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str = "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

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
    let from_python = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        let app: Py<PyAny> =
            PyModule::from_code(py, PY_RANDOM_NUMBER_GENERATOR_SCRIPT, "", "")?.getattr("generate")?.into();
        app.call(py, (seed, random_numbers_count), Option::None)
    });

    let result = from_python
        .map_err(|_| Error::PyModuleError)?
        .to_string()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(",")
        .map(|el| el.trim().parse::<u128>().unwrap())
        .collect::<Vec<u128>>();

    Ok(result)
}

pub(crate) fn load_cairo_0_contract_class(path: &str) -> DevnetResult<LegacyContractClass> {
    let contract_class_str = fs::read_to_string(&path).map_err(crate::error::Error::IOError)?;
    let contract_class: LegacyContractClass =
        serde_json::from_str(&contract_class_str).map_err(JsonError::SerdeJsonError)?;

    Ok(contract_class)
}

pub(crate) fn compute_cairo_0_class_hash(contract_class: &ContractClass) -> DevnetResult<ClassHash> {
    let class_hash_felt_252 =
        compute_deprecated_class_hash(contract_class).map_err(Error::StarknetInRustContractAddressError)?;

    Ok(Felt(class_hash_felt_252.to_be_bytes()))
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::core::contract_address::{
        starknet_contract_address::compute_deprecated_class_hash,
        v2::starknet_sierra_contract_address::compute_sierra_class_hash,
    };
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;

    use crate::utils::{compute_cairo_0_class_hash, CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};

    use super::{generate_u128_random_numbers, get_bytes_from_u32, load_cairo_0_contract_class};
    use crate::types::Felt;

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
    fn cairo_0_loading_should_be_successful() {
        assert!(load_cairo_0_contract_class(CAIRO_0_ACCOUNT_CONTRACT_PATH).is_ok());
    }

    #[test]
    fn correct_cairo_0_class_hash_computation() {
        let contract_class = load_cairo_0_contract_class(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let class_hash = contract_class.class_hash().unwrap();
        assert_eq!(Felt::from(class_hash), Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap());
    }
}
