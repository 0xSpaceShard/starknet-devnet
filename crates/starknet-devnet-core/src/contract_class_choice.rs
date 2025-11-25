use std::str::FromStr;

use starknet_rs_core::types::Felt;
use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_class::deprecated::json_contract_class::Cairo0Json;
use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
use starknet_types::traits::HashProducer;

use crate::constants::{CAIRO_0_ACCOUNT_CONTRACT, CAIRO_1_ACCOUNT_CONTRACT_SIERRA};
use crate::error::DevnetResult;

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum AccountContractClassChoice {
    Cairo0,
    Cairo1,
}

impl AccountContractClassChoice {
    pub fn get_class_wrapper(&self) -> DevnetResult<AccountClassWrapper> {
        Ok(match self {
            AccountContractClassChoice::Cairo0 => {
                let contract_json = Cairo0Json::raw_json_from_json_str(CAIRO_0_ACCOUNT_CONTRACT)?;
                let contract_class = Cairo0ContractClass::RawJson(contract_json);

                AccountClassWrapper {
                    class_hash: contract_class.generate_hash()?,
                    contract_class: ContractClass::Cairo0(contract_class),
                    class_metadata: "OpenZeppelin 0.5.1",
                }
            }
            AccountContractClassChoice::Cairo1 => {
                let contract_class = ContractClass::Cairo1(
                    ContractClass::cairo_1_from_sierra_json_str(CAIRO_1_ACCOUNT_CONTRACT_SIERRA)?,
                );
                AccountClassWrapper {
                    class_hash: contract_class.generate_hash()?,
                    contract_class,
                    class_metadata: "OpenZeppelin 1.0.0",
                }
            }
        })
    }
}
#[derive(Clone, Debug)]
pub struct AccountClassWrapper {
    pub contract_class: ContractClass,
    pub class_hash: Felt,
    pub class_metadata: &'static str,
}

impl FromStr for AccountClassWrapper {
    type Err = crate::error::Error;

    fn from_str(path_candidate: &str) -> Result<Self, Self::Err> {
        // load artifact
        let contract_class = ContractClass::cairo_1_from_sierra_json_str(
            std::fs::read_to_string(path_candidate)?.as_str(),
        )?;

        // check that artifact is really account
        let execute_selector = get_selector_from_name("__execute__")
            .map_err(|err| crate::error::Error::UnexpectedInternalError { msg: err.to_string() })?;
        let validate_selector = get_selector_from_name("__validate__")
            .map_err(|err| crate::error::Error::UnexpectedInternalError { msg: err.to_string() })?;
        let mut has_execute = false;
        let mut has_validate = false;
        for entry_point in &contract_class.entry_points_by_type.external {
            let selector: Felt = (&entry_point.selector).into();
            has_execute |= selector == execute_selector;
            has_validate |= selector == validate_selector;
        }

        if !has_execute || !has_validate {
            let msg = format!(
                "Not a valid Sierra account artifact; has __execute__: {has_execute}; has \
                 __validate__: {has_validate}"
            );
            return Err(crate::error::Error::ContractClassLoadError(msg));
        }

        // generate the hash and return
        let contract_class = ContractClass::Cairo1(contract_class);
        let class_hash = contract_class.generate_hash()?;
        Ok(Self { contract_class, class_hash, class_metadata: "Custom" })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use clap::ValueEnum;
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::traits::HashProducer;

    use super::AccountContractClassChoice;
    use crate::constants::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
        CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
    };
    use crate::contract_class_choice::AccountClassWrapper;

    #[test]
    fn all_methods_work_with_all_options() {
        for implementation in AccountContractClassChoice::value_variants().iter() {
            let AccountClassWrapper { contract_class, class_hash, class_metadata } =
                implementation.get_class_wrapper().unwrap();
            let generated_hash = contract_class.generate_hash().unwrap();
            assert_eq!(generated_hash, class_hash);
            assert!(class_metadata.starts_with("OpenZeppelin"));
        }
    }

    #[test]
    fn correct_hash_calculated() {
        assert_eq!(
            AccountContractClassChoice::Cairo0.get_class_wrapper().unwrap().class_hash,
            felt_from_prefixed_hex(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap()
        );

        assert_eq!(
            AccountContractClassChoice::Cairo1.get_class_wrapper().unwrap().class_hash,
            felt_from_prefixed_hex(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
        )
    }

    #[test]
    fn correct_metadata() {
        assert_eq!(
            AccountContractClassChoice::Cairo0.get_class_wrapper().unwrap().class_metadata,
            "OpenZeppelin 0.5.1"
        );
        assert_eq!(
            AccountContractClassChoice::Cairo1.get_class_wrapper().unwrap().class_metadata,
            "OpenZeppelin 1.0.0"
        );

        let custom_class =
            AccountClassWrapper::from_str(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH).unwrap();
        assert_eq!(custom_class.class_metadata, "Custom");
    }
}
