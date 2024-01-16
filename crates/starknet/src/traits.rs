use blockifier::state::state_api::{State, StateReader};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{Balance, ClassHash};

use crate::account::FeeToken;
use crate::error::DevnetResult;

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
    fn deploy(&self, state: &mut (impl State + StateReader)) -> DevnetResult<()>;
    fn get_address(&self) -> ContractAddress;
}

/// This trait sets the interface for the account
pub trait Accounted {
    /// Set initial balance for the account in ETH and STRK token
    ///
    /// # Arguments
    /// `state` - state of the devnet
    fn set_initial_balance(&self, state: &mut impl State) -> DevnetResult<()>;

    /// Get balance of the account. In `FeeToken` token
    ///
    /// # Arguments
    /// `state` - state of the devnet
    /// `token` - enum `FeeToken` to get balance in
    fn get_balance(&self, state: &mut impl StateReader, token: FeeToken) -> DevnetResult<Balance>;
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
