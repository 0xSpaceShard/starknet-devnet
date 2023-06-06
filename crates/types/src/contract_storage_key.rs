use super::{contract_address::ContractAddress, felt::Felt};

#[derive(Debug, Default, Clone, Copy)]
pub struct ContractStorageKey(ContractAddress, Felt);
