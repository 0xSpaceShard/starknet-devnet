use crate::traits::ToHexString;

use super::felt::Felt;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ContractAddress(Felt);

impl ContractAddress {
    pub fn new(felt: Felt) -> Self {
        Self(felt)
    }
}

impl From<starknet_api::core::ContractAddress> for ContractAddress {
    fn from(value: starknet_api::core::ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl ToHexString for ContractAddress {
    fn to_prefixed_hex_str(&self) -> String {
        self.0.to_prefixed_hex_str()
    }

    fn to_nonprefixed_hex_str(&self) -> String {
        self.0.to_nonprefixed_hex_str()
    }
}
