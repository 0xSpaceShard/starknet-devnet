use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::error::Error;
use starknet_types::felt::{Balance, ClassHash, Felt};
use starknet_types::DevnetResult;

/// This trait should be implemented by structures that internally have collections and each element could be found by a hash
pub trait HashIdentified {
    type Element;
    type Hash;

    fn get_by_hash(&self, hash: Self::Hash) -> Option<&Self::Element>;
}

/// This trait sets the interface for the account
pub trait Accounted {
    fn deploy(&self, state: &mut impl StateChanger) -> Result<(), Error>;
    fn set_initial_balance(&self, state: &mut impl StateChanger) -> DevnetResult<()>;
    fn get_balance(&self, state: &mut impl StateExtractor) -> DevnetResult<Balance>;
    fn get_address(&self) -> ContractAddress;
}

/// Interface for modifying the state
pub trait StateChanger {
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()>;
    fn deploy_contract(
        &mut self,
        address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()>;
    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> DevnetResult<()>;
    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()>;
    fn is_contract_declared(&mut self, class_hash: &ClassHash) -> DevnetResult<bool>;
    fn apply_cached_state(&mut self) -> DevnetResult<()>;
}

/// Interface for extracting data from the state
pub trait StateExtractor {
    fn get_storage(&mut self, storage_key: ContractStorageKey) -> DevnetResult<Felt>;
}

/// This trait should be implemented by structures that generate accounts
pub trait AccountGenerator {
    type Acc: Accounted;
    fn generate_accounts(
        &mut self,
        number_of_accounts: u8,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<&Vec<Self::Acc>>;
}
