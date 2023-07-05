use super::felt::Felt;
use crate::error::Error;
use crate::patricia_key::PatriciaKey;
use crate::traits::ToHexString;
use crate::DevnetResult;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ContractAddress(pub(crate) PatriciaKey);

impl ContractAddress {
    pub fn new(felt: Felt) -> DevnetResult<Self> {
        Ok(Self(PatriciaKey::new(felt)?))
    }
}

impl From<starknet_api::core::ContractAddress> for ContractAddress {
    fn from(value: starknet_api::core::ContractAddress) -> Self {
        Self(value.0.into())
    }
}

impl TryFrom<ContractAddress> for starknet_api::core::ContractAddress {
    type Error = Error;

    fn try_from(value: ContractAddress) -> DevnetResult<Self> {
        let patricia_key: starknet_api::core::PatriciaKey = value.0.try_into()?;
        Ok(starknet_api::core::ContractAddress(patricia_key))
    }
}

impl TryFrom<ContractAddress> for starknet_in_rust::utils::Address {
    type Error = Error;

    fn try_from(value: ContractAddress) -> DevnetResult<Self> {
        let felt_252 = cairo_felt::Felt252::from(value.0 .0);
        Ok(Self(felt_252))
    }
}

impl TryFrom<&ContractAddress> for starknet_in_rust::utils::Address {
    type Error = Error;

    fn try_from(value: &ContractAddress) -> DevnetResult<Self> {
        let felt_252 = cairo_felt::Felt252::from(&value.0 .0);
        Ok(Self(felt_252))
    }
}

impl TryFrom<starknet_in_rust::utils::Address> for ContractAddress {
    type Error = Error;

    fn try_from(value: starknet_in_rust::utils::Address) -> DevnetResult<Self> {
        Self::new(Felt::from(value.0))
    }
}

impl ToHexString for ContractAddress {
    fn to_prefixed_hex_str(&self) -> String {
        self.0 .0.to_prefixed_hex_str()
    }

    fn to_nonprefixed_hex_str(&self) -> String {
        self.0 .0.to_nonprefixed_hex_str()
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::utils::Address;

    use super::ContractAddress;
    use crate::contract_address::test_utils;
    use crate::utils::test_utils::dummy_felt;

    #[test]
    fn correct_convertion_to_starknet_in_rust_address() {
        let address = ContractAddress::new(dummy_felt()).unwrap();
        let sn_address: Address = TryFrom::try_from(&address).unwrap();

        assert!(test_utils::is_equal(&address, &sn_address));
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use starknet_in_rust::utils::Address;

    use super::ContractAddress;

    pub fn is_equal(lhs: &ContractAddress, rhs: &Address) -> bool {
        lhs.0 .0.bytes() == rhs.0.to_be_bytes()
    }
}
