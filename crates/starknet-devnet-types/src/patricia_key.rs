use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::core::{CONTRACT_ADDRESS_DOMAIN_SIZE, PATRICIA_KEY_UPPER_BOUND};
use starknet_rs_core::types::FieldElement;
use starknet_rs_core::utils::NonAsciiNameError;

use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use crate::serde_helpers::hex_string::{
    deserialize_to_prefixed_patricia_key, serialize_patricia_key_to_prefixed_hex,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatriciaKey(pub(crate) Felt);

impl PatriciaKey {
    pub fn new(felt: Felt) -> DevnetResult<Self> {
        if *CONTRACT_ADDRESS_DOMAIN_SIZE < felt.into() {
            return Err(Error::StarknetApiError(starknet_api::StarknetApiError::OutOfRange {
                string: format!("[0x0, {PATRICIA_KEY_UPPER_BOUND})"),
            }));
        }

        Ok(PatriciaKey(felt))
    }

    pub fn to_felt(&self) -> Felt {
        self.0
    }
}

impl Serialize for PatriciaKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_patricia_key_to_prefixed_hex(self, serializer)
    }
}

impl<'de> Deserialize<'de> for PatriciaKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_to_prefixed_patricia_key(deserializer)
    }
}

impl From<starknet_api::core::PatriciaKey> for PatriciaKey {
    fn from(value: starknet_api::core::PatriciaKey) -> Self {
        Self(value.into())
    }
}

impl TryFrom<PatriciaKey> for starknet_api::core::PatriciaKey {
    type Error = Error;

    fn try_from(value: PatriciaKey) -> Result<Self, Self::Error> {
        let stark_hash: starknet_api::hash::StarkFelt = value.0.into();
        Ok(starknet_api::core::PatriciaKey::try_from(stark_hash)?)
    }
}

impl TryFrom<Felt> for PatriciaKey {
    type Error = Error;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<PatriciaKey> for starknet_api::state::StorageKey {
    type Error = Error;

    fn try_from(value: PatriciaKey) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl TryFrom<FieldElement> for PatriciaKey {
    type Error = Error; // Replace with the appropriate error type

    fn try_from(element: FieldElement) -> Result<Self, Self::Error> {
        let bytes = Felt::new(element.to_bytes_be())?; // Replace with the appropriate method to convert FieldElement to bytes
        PatriciaKey::new(bytes) // Replace with the appropriate error handling
    }
}

impl TryFrom<Result<FieldElement, NonAsciiNameError>> for PatriciaKey {
    type Error = Error;

    fn try_from(value: Result<FieldElement, NonAsciiNameError>) -> Result<Self, Self::Error> {
        match value {
            Ok(field_element) => PatriciaKey::try_from(field_element),
            Err(_) => Err(Error::ProgramError),
        }
    }
}

pub type StorageKey = PatriciaKey;

#[cfg(test)]
mod tests {
    use super::PatriciaKey;
    use crate::felt::Felt;
    use starknet_rs_core::types::FieldElement;
    use crate::contract_address::ContractAddress;
    use starknet_rs_core::utils::NonAsciiNameError;

    #[test]
    fn creation_of_patricia_key_should_be_successful() {
        assert!(
            PatriciaKey::new(
                Felt::from_prefixed_hex_str(
                    "0x7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                )
                .unwrap()
            )
            .is_ok()
        );
    }

    #[test]
    fn patricia_key_with_too_large_felt_should_return_error() {
        let result = PatriciaKey::new(
            Felt::from_prefixed_hex_str(
                "0x800000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn patricia_key_try_from_field_element_succeeds() {
        let account_address = ContractAddress::new(Felt::from(111)).unwrap();
        let field_element = FieldElement::from(account_address); 
        let result = PatriciaKey::try_from(field_element);
        assert!(result.is_ok());
        let patricia_key = result.unwrap();
        assert_eq!(patricia_key.to_felt(), Felt::from(111));

    }

    #[test]
    fn test_try_from_result() {
        let account_address = ContractAddress::new(Felt::from(111)).unwrap();
        let field_element = FieldElement::from(account_address); 
        let result: Result<FieldElement, NonAsciiNameError> = Ok(field_element);
        let patricia_key = PatriciaKey::try_from(result);
        assert!(patricia_key.is_ok());
        assert_eq!(patricia_key.unwrap().to_felt(), Felt::from(111));
    }
}
