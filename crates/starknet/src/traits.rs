use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::rpc::felt::{Balance, ClassHash, Felt};

use crate::error::Result;
use crate::state::state_diff::StateDiff;

/// This trait should be implemented by structures that internally have collections and each element
/// could be found by a hash
pub trait HashIdentified {
    type Element;
    type Hash;

    fn get_by_hash(&self, hash: Self::Hash) -> Option<&Self::Element>;
}

pub trait HashIdentifiedMut {
    type Element;
    type Hash;

    fn get_by_hash_mut(&mut self, hash: &Self::Hash) -> Option<&mut Self::Element>;
}

pub trait Deployed {
    fn deploy(&self, state: &mut (impl StateChanger + StateExtractor)) -> Result<()>;
    fn get_address(&self) -> ContractAddress;
}

/// This trait sets the interface for the account
pub trait Accounted {
    fn set_initial_balance(&self, state: &mut impl StateChanger) -> Result<()>;
    fn get_balance(&self, state: &mut impl StateExtractor) -> Result<Balance>;
}

/// Interface for modifying the state
pub trait StateChanger {
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> Result<()>;
    fn deploy_contract(&mut self, address: ContractAddress, class_hash: ClassHash) -> Result<()>;
    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> Result<()>;
    fn increment_nonce(&mut self, address: ContractAddress) -> Result<()>;
    // apply state_diff to "persistent" state
    fn apply_state_difference(&mut self, state_diff: StateDiff) -> Result<()>;
}

/// Interface for extracting data from the state
pub trait StateExtractor {
    fn get_storage(&self, storage_key: ContractStorageKey) -> Result<Felt>;
    fn is_contract_declared(&mut self, class_hash: &ClassHash) -> bool;
    fn is_contract_deployed(&self, address: &ContractAddress) -> bool;
    fn get_class_hash_at_contract_address(
        &mut self,
        address: &ContractAddress,
    ) -> Result<ClassHash>;
    fn extract_state_diff_from_pending_state(&self) -> Result<StateDiff>;
}

/// This trait should be implemented by structures that generate accounts
pub trait AccountGenerator {
    type Acc: Accounted;
    fn generate_accounts(
        &mut self,
        number_of_accounts: u8,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> Result<&Vec<Self::Acc>>;
}
