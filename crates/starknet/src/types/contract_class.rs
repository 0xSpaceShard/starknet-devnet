use std::path::PathBuf;

use crate::{
    error::{Error, JsonError},
    traits::HashProducer,
};

use super::{felt::Felt, DevnetResult};
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContractClass(StarknetInRustContractClass);

impl ContractClass {
    pub fn load_from_file(path: &str) -> DevnetResult<Self> {
        let contract_path = PathBuf::from(path);
        let starknet_in_rust_class = if contract_path.exists() {
            StarknetInRustContractClass::try_from(PathBuf::from(path)).map_err(|_| {
                Error::JsonError(JsonError::Custom { msg: "unable to load contract class from file".to_string() })
            })
        } else {
            Err(Error::IOError(std::io::Error::new(std::io::ErrorKind::NotFound, "path doesnt exist")))
        }?;

        Ok(Self(starknet_in_rust_class))
    }
}

impl From<ContractClass> for StarknetInRustContractClass {
    fn from(value: ContractClass) -> Self {
        value.0
    }
}

impl From<&ContractClass> for StarknetInRustContractClass {
    fn from(value: &ContractClass) -> Self {
        value.0.clone()
    }
}

impl HashProducer for ContractClass {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        crate::utils::compute_cairo_0_class_hash(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{constants, error::Error};

    use super::ContractClass;

    #[test]
    fn load_contract_class_from_file_successfully() {
        assert!(ContractClass::load_from_file(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH).is_ok())
    }

    #[test]
    fn load_contract_class_from_file_failed_with_path_not_found() {
        let errored_result = ContractClass::load_from_file("/usr/local_dev/not_found.json");
        if let Some(err) = errored_result.err() {
            if let Error::IOError(inner_err) = err {
                assert_eq!(inner_err.kind(), std::io::ErrorKind::NotFound);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }
}
