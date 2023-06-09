use starknet_api::core::{CONTRACT_ADDRESS_DOMAIN_SIZE, PATRICIA_KEY_UPPER_BOUND};

use crate::error::Error;
use crate::felt::Felt;
use crate::DevnetResult;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

impl From<starknet_api::core::PatriciaKey> for PatriciaKey {
    fn from(value: starknet_api::core::PatriciaKey) -> Self {
        Self(value.into())
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
        assert!(PatriciaKey::new(
            Felt::from_prefixed_hex_str(
                "0x7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            )
            .unwrap()
        )
        .is_ok());
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
