use starknet_types::{felt::Felt, contract_address::ContractAddress};

use super::models::{FeltHex, ContractAddressHex};

impl From<&Felt> for FeltHex {
    fn from(value: &Felt) -> Self {
        Self(*value)
    }
}

impl From<Felt> for FeltHex {
    fn from(value: Felt) -> Self {
        Self(value)
    }
}

impl From<ContractAddress> for ContractAddressHex {
    fn from(value: ContractAddress) -> Self {
        Self(value)
    }
}

impl From<&ContractAddress> for ContractAddressHex {
    fn from(value: &ContractAddress) -> Self {
        Self(*value)
    }
}

pub(crate) fn into_vec<'a, T, U>(value: &'a Vec<T>) -> Vec<U>
where
    U: std::convert::From<&'a T>,
{
    value.iter().map(|x| U::from(x)).collect()
}
