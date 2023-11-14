use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::core::{CONTRACT_ADDRESS_DOMAIN_SIZE, PATRICIA_KEY_UPPER_BOUND};

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

pub type StorageKey = PatriciaKey;

#[cfg(test)]
mod tests {
    use super::PatriciaKey;
    use crate::felt::Felt;

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
}
