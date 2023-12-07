use std::fmt::LowerHex;
use std::str::FromStr;

use cairo_felt::Felt252;
use num_bigint::BigUint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_api::serde_utils::{bytes_from_hex_str, hex_str_from_bytes};
use starknet_api::StarknetApiError;

use crate::contract_address::ContractAddress;
use crate::error::{ConversionError, DevnetResult, Error};
use crate::serde_helpers::hex_string::{
    deserialize_prefixed_hex_string_to_felt, serialize_to_prefixed_hex,
};
use crate::traits::{ToDecimalString, ToHexString};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Felt(pub(crate) [u8; 32]);

impl Serialize for Felt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_to_prefixed_hex(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Felt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_prefixed_hex_string_to_felt(deserializer)
    }
}

impl Felt {
    pub fn new(bytes: [u8; 32]) -> DevnetResult<Self> {
        if bytes[0] < 0x10 {
            return Ok(Self(bytes));
        }
        Err(Error::StarknetApiError(StarknetApiError::OutOfRange {
            string: hex_str_from_bytes::<32, true>(bytes),
        }))
    }

    pub fn to_field_element(&self) -> DevnetResult<starknet_rs_ff::FieldElement> {
        starknet_rs_ff::FieldElement::from_bytes_be(&self.0)
            .map_err(|_| Error::ConversionError(crate::error::ConversionError::FromByteArrayError))
    }

    pub fn from_prefixed_hex_str(hex_str: &str) -> DevnetResult<Self> {
        let bytes = bytes_from_hex_str::<32, true>(hex_str).map_err(|err| {
            Error::StarknetApiError(starknet_api::StarknetApiError::InnerDeserialization(err))
        })?;

        Self::new(bytes)
    }

    pub fn bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }
}

impl ToHexString for Felt {
    fn to_prefixed_hex_str(&self) -> String {
        hex_str_from_bytes::<32, true>(self.0)
    }

    fn to_nonprefixed_hex_str(&self) -> String {
        hex_str_from_bytes::<32, false>(self.0)
    }
}

impl ToDecimalString for Felt {
    fn to_decimal_string(&self) -> String {
        let bigint = BigUint::from_bytes_be(&self.bytes());
        bigint.to_str_radix(10)
    }
}

impl From<Felt> for starknet_rs_ff::FieldElement {
    fn from(value: Felt) -> Self {
        starknet_rs_ff::FieldElement::from_bytes_be(&value.0)
            .expect("Convert Felt to FieldElement, should be the same")
    }
}

impl From<starknet_rs_ff::FieldElement> for Felt {
    fn from(value: starknet_rs_ff::FieldElement) -> Self {
        Self(value.to_bytes_be())
    }
}

impl From<u128> for Felt {
    fn from(value: u128) -> Self {
        let le_part: [u8; 16] = value.to_be_bytes();
        let byte_arr: [u8; 32] = [[0u8; 16], le_part].concat().try_into().unwrap();
        Self(byte_arr)
    }
}

impl TryFrom<Felt> for u128 {
    type Error = Error;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        let ff = starknet_rs_ff::FieldElement::from(value);
        ff.try_into().map_err(|_| {
            Error::ConversionError(ConversionError::OutOfRangeError(
                "Felt is too large to be converted into u128 value".to_string(),
            ))
        })
    }
}

impl From<ContractAddress> for Felt {
    fn from(value: ContractAddress) -> Self {
        value.0.0
    }
}

impl From<starknet_api::hash::StarkFelt> for Felt {
    fn from(value: starknet_api::hash::StarkFelt) -> Self {
        let arr: [u8; 32] = value.bytes().try_into().expect("slice of incorrect length");
        Self(arr)
    }
}

impl From<Felt> for starknet_api::hash::StarkFelt {
    fn from(value: Felt) -> Self {
        starknet_api::hash::StarkFelt::new(value.0).expect("Invalid bytes")
    }
}

impl From<&Felt> for starknet_api::hash::StarkFelt {
    fn from(value: &Felt) -> Self {
        starknet_api::hash::StarkFelt::new(value.0).expect("Invalid bytes")
    }
}

impl From<starknet_api::core::ClassHash> for Felt {
    fn from(value: starknet_api::core::ClassHash) -> Self {
        Felt::from(value.0)
    }
}

impl From<Felt> for starknet_api::core::ClassHash {
    fn from(value: Felt) -> Self {
        Self(starknet_api::hash::StarkFelt::from(value))
    }
}

impl From<Felt> for starknet_api::core::CompiledClassHash {
    fn from(value: Felt) -> Self {
        Self(starknet_api::hash::StarkFelt::from(value))
    }
}

impl From<cairo_felt::Felt252> for Felt {
    fn from(value: cairo_felt::Felt252) -> Self {
        Self(value.to_be_bytes())
    }
}

impl From<&cairo_felt::Felt252> for Felt {
    fn from(value: &Felt252) -> Self {
        Self(value.to_be_bytes())
    }
}

impl From<Felt> for cairo_felt::Felt252 {
    fn from(value: Felt) -> Self {
        Self::from_bytes_be(&value.0)
    }
}

impl From<&Felt> for cairo_felt::Felt252 {
    fn from(value: &Felt) -> Self {
        Self::from_bytes_be(&value.0)
    }
}

impl From<starknet_api::core::PatriciaKey> for Felt {
    fn from(value: starknet_api::core::PatriciaKey) -> Self {
        let arr: [u8; 32] = value.key().bytes().try_into().expect("slice of incorrect length");
        Self(arr)
    }
}

impl TryFrom<Felt> for starknet_api::core::PatriciaKey {
    type Error = crate::error::Error;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        Ok(starknet_api::core::PatriciaKey::try_from(starknet_api::hash::StarkFelt::from(value))?)
    }
}

impl From<Felt> for starknet_api::block::BlockHash {
    fn from(value: Felt) -> Self {
        Self(value.into())
    }
}

impl From<starknet_api::block::BlockHash> for Felt {
    fn from(value: starknet_api::block::BlockHash) -> Self {
        value.0.into()
    }
}

impl TryFrom<BigUint> for Felt {
    type Error = crate::error::Error;

    fn try_from(value: BigUint) -> DevnetResult<Self> {
        let hex_str = format!("0x{}", value.to_str_radix(16));
        Felt::from_prefixed_hex_str(&hex_str)
    }
}

impl From<Felt> for BigUint {
    fn from(felt: Felt) -> Self {
        BigUint::from_str(&felt.to_decimal_string()).expect("Should never fail: felt is 251 bits")
    }
}

impl LowerHex for Felt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_prefixed_hex_str().as_str())
    }
}

pub type Nonce = Felt;
pub type TransactionVersion = Felt;
pub type TransactionSignature = Vec<Felt>;
pub type CompiledClassHash = Felt;
pub type EntryPointSelector = Felt;
pub type Calldata = Vec<Felt>;
pub type ContractAddressSalt = Felt;
pub type BlockHash = Felt;
pub type TransactionHash = Felt;
pub type ClassHash = Felt;
pub type Key = Felt;
pub type Balance = Felt;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use num_bigint::BigUint;

    use super::Felt;
    use crate::traits::ToDecimalString;
    #[test]
    fn correct_conversion_from_hex_str_to_felt() {
        assert!(
            Felt::from_prefixed_hex_str(
                "0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854"
            )
            .is_ok()
        )
    }

    #[test]
    fn correct_value_after_hex_str_to_felt() {
        let felt = Felt::from_prefixed_hex_str("0xAA").unwrap();
        assert_eq!(felt.0[31], 170);
    }

    #[test]
    fn correct_conversion_from_bigint_to_felt() {
        let bigint = BigUint::from(123456u128);
        assert_eq!(Felt::try_from(bigint).unwrap(), Felt::from_prefixed_hex_str("0x1e240").unwrap())
    }

    #[test]
    /// 2**250 + 1
    fn correct_conversion_from_decimal_string_to_felt() {
        let s = "1809251394333065553493296640760748560207343510400633813116524750123642650625";
        let bigint = BigUint::from_str(s).unwrap();
        assert_eq!(
            Felt::try_from(bigint).unwrap(),
            Felt::from_prefixed_hex_str(
                "0x400000000000000000000000000000000000000000000000000000000000001"
            )
            .unwrap()
        )
    }

    #[test]
    /// 2**250 + 1
    fn correct_conversion_from_felt_to_decimal_string() {
        assert_eq!(
            Felt::from_prefixed_hex_str(
                "0x400000000000000000000000000000000000000000000000000000000000001"
            )
            .unwrap()
            .to_decimal_string(),
            "1809251394333065553493296640760748560207343510400633813116524750123642650625"
        );
    }
}
