use std::fmt::LowerHex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_rs_core::types::Felt;

use crate::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
use crate::error::DevnetResult;
use crate::patricia_key::{PATRICIA_KEY_ZERO, PatriciaKey};
use crate::rpc::transaction_receipt::FeeUnit;
use crate::serde_helpers::hex_string::{
    deserialize_to_prefixed_contract_address, serialize_contract_address_to_prefixed_hex,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractAddress(pub(crate) PatriciaKey);

impl ContractAddress {
    pub fn new(felt: Felt) -> DevnetResult<Self> {
        Ok(Self(PatriciaKey::new(felt)?))
    }

    /// Constructs a zero address
    pub fn zero() -> Self {
        Self(PATRICIA_KEY_ZERO)
    }

    pub fn to_fixed_hex_string(&self) -> String {
        self.0.0.to_fixed_hex_string()
    }

    pub fn from_feeunit(unit: &FeeUnit) -> Self {
        let erc20_contract_address = match unit {
            FeeUnit::WEI => ETH_ERC20_CONTRACT_ADDRESS,
            FeeUnit::FRI => STRK_ERC20_CONTRACT_ADDRESS,
        };

        // We can safely use unchecked here because these addresses are known to be valid
        Self(PatriciaKey::new_unchecked(erc20_contract_address))
    }
}

impl Serialize for ContractAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_contract_address_to_prefixed_hex(self, serializer)
    }
}

impl<'de> Deserialize<'de> for ContractAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_to_prefixed_contract_address(deserializer)
    }
}

impl From<starknet_api::core::ContractAddress> for ContractAddress {
    fn from(value: starknet_api::core::ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl From<ContractAddress> for starknet_api::core::ContractAddress {
    fn from(value: ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl From<ContractAddress> for Felt {
    fn from(value: ContractAddress) -> Self {
        value.0.0
    }
}

impl LowerHex for ContractAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.0.to_hex_string().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_unchecked_vs_new_for_feeaddress() {
        // For ETH address
        let patricia_key_checked = PatriciaKey::new(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        let patricia_key_unchecked = PatriciaKey::new_unchecked(ETH_ERC20_CONTRACT_ADDRESS);
        assert_eq!(
            patricia_key_checked, patricia_key_unchecked,
            "PatriciaKey::new and PatriciaKey::new_unchecked should produce the same result for \
             valid ETH address"
        );

        // For STRK address
        let patricia_key_checked = PatriciaKey::new(STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        let patricia_key_unchecked = PatriciaKey::new_unchecked(STRK_ERC20_CONTRACT_ADDRESS);
        assert_eq!(
            patricia_key_checked, patricia_key_unchecked,
            "PatriciaKey::new and PatriciaKey::new_unchecked should produce the same result for \
             valid STRK address"
        );
    }
}
