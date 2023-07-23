use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;

use super::models::{ContractAddressHex, FeltHex};

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

pub(crate) fn into_vec<'a, T, U>(value: &'a [T]) -> Vec<U>
where
    U: std::convert::From<&'a T>,
{
    value.iter().map(|x| U::from(x)).collect()
}
