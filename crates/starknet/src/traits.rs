use starknet_types::{contract_class::ContractClass, error::Error, felt::ClassHash, DevnetResult};

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
    type Acc: Accounted;
    fn generate_accounts(
        &self,
        number_of_accounts: u8,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<Vec<Self::Acc>>;
}
