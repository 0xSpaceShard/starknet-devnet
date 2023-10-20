use starknet_core::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_PATH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
};
use starknet_types::contract_class::{Cairo0ContractClass, Cairo0Json, ContractClass};
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum AccountContractClassChoice {
    Cairo0,
    Cairo1,
}

impl AccountContractClassChoice {
    pub fn get_path(&self) -> &str {
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

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::AccountContractClassChoice;

    #[test]
    fn all_methods_work_with_all_options() {
        for implementation in AccountContractClassChoice::value_variants().iter() {
            assert!(implementation.get_class().is_ok());
            assert!(implementation.get_hash().is_ok());
        }
    }
}
