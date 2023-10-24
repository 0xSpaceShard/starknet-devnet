use std::str::FromStr;

use starknet_core::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_PATH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
};
use starknet_rs_core::types::FieldElement;
use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_class::{Cairo0ContractClass, Cairo0Json, ContractClass};
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum AccountContractClassChoice {
    Cairo0,
    Cairo1,
}

// TODO refactor to rely on AccountClassWrapper
impl AccountContractClassChoice {
    fn get_path(&self) -> &str {
        match self {
            AccountContractClassChoice::Cairo0 => CAIRO_0_ACCOUNT_CONTRACT_PATH,
            AccountContractClassChoice::Cairo1 => CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        }
    }

    pub fn get_class(&self) -> Result<ContractClass, anyhow::Error> {
        let contract_class = match self {
            Self::Cairo0 => ContractClass::Cairo0(Cairo0ContractClass::RawJson(
                Cairo0Json::raw_json_from_path(self.get_path())?,
            )),
            Self::Cairo1 => ContractClass::Cairo1(ContractClass::cairo_1_from_sierra_json_str(
                std::fs::read_to_string(self.get_path())?.as_str(),
            )?),
        };
        Ok(contract_class)
    }

    pub fn get_hash(&self) -> Result<Felt, anyhow::Error> {
        let hash = match self {
            AccountContractClassChoice::Cairo0 => {
                Cairo0Json::raw_json_from_path(self.get_path())?.generate_hash()?
            }
            AccountContractClassChoice::Cairo1 => {
                let contract_class_str = std::fs::read_to_string(self.get_path())?;
                let account_contract_class = ContractClass::Cairo1(
                    ContractClass::cairo_1_from_sierra_json_str(&contract_class_str)?,
                );
                account_contract_class.generate_hash()?
            }
        };
        Ok(hash)
    }
}

#[derive(Clone, Debug)]
pub struct AccountClassWrapper {
    pub contract_class: ContractClass,
    pub class_hash: Felt,
}

impl FromStr for AccountClassWrapper {
    type Err = anyhow::Error;

    fn from_str(path_candidate: &str) -> Result<Self, Self::Err> {
        // load artifact
        let contract_class = ContractClass::cairo_1_from_sierra_json_str(
            std::fs::read_to_string(path_candidate)?.as_str(),
        )?;

        // check that artifact is really account
        let execute_selector: FieldElement = get_selector_from_name("__execute__").unwrap();
        let validate_selector: FieldElement = get_selector_from_name("__validate__").unwrap();
        let mut has_execute = false;
        let mut has_validate = false;
        for entry_point in contract_class.entry_points_by_type.external.iter() {
            let selector_bytes = entry_point.selector.to_bytes_be();
            match FieldElement::from_byte_slice_be(&selector_bytes) {
                Ok(selector) if selector == execute_selector => has_execute = true,
                Ok(selector) if selector == validate_selector => has_validate = true,
                _ => (),
            }
        }
        if !has_execute || !has_validate {
            let msg = format!(
                "Not a valid Sierra account artifact; has __execute__: {has_execute}; has \
                 __validate__: {has_validate}"
            );
            return Err(anyhow::Error::msg(msg));
        }

        // generate the hash and return
        let contract_class = ContractClass::Cairo1(contract_class);
        let class_hash = contract_class.generate_hash()?;
        Ok(Self { contract_class, class_hash })
    }
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;
    use starknet_core::constants::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
    };
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use super::AccountContractClassChoice;

    #[test]
    fn all_methods_work_with_all_options() {
        for implementation in AccountContractClassChoice::value_variants().iter() {
            let contract_class = implementation.get_class().unwrap();
            let generated_hash = contract_class.generate_hash().unwrap();
            assert_eq!(generated_hash, implementation.get_hash().unwrap());
        }
    }

    #[test]
    fn correct_hash_calculated() {
        assert_eq!(
            AccountContractClassChoice::Cairo0.get_hash().unwrap(),
            Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap()
        );

        assert_eq!(
            AccountContractClassChoice::Cairo1.get_hash().unwrap(),
            Felt::from_prefixed_hex_str(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
        )
    }
}
