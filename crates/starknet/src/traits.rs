use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass;

use crate::{
    error::Error,
    types::{ClassHash, DevnetResult},
};

pub trait HashIdentified {
    type Element;
    type Hash;

    fn get_by_hash(&self, hash: Self::Hash) -> Self::Element;
}

pub trait Accounted {
    fn deploy(&self, state: impl StateChanger) -> Result<(), Error>;
    fn declare(&self, state: &mut impl StateChanger) -> Result<(), Error>;
}

pub trait StateChanger {
    fn declare_contract_class(&mut self, hash: ClassHash, contract_class: &ContractClass) -> Result<(), Error>;
}

pub trait AccountGenerator {
    type Acc;
    fn generate_accounts(&self, number_of_accounts: u8) -> DevnetResult<Vec<Self::Acc>>
    where
        Self::Acc: Accounted;
}
