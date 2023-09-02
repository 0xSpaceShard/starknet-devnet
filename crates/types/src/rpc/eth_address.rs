use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use cairo_felt::Felt252;
use serde::{Deserialize, Serialize};
use starknet_in_rust::utils::Address as SirAddress;
use starknet_rs_core::types::EthAddress;
use starknet_rs_ff::FieldElement;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct EthAddressWrapper {
    pub inner: EthAddress,
}

impl Serialize for EthAddressWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EthAddressWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(EthAddressWrapper { inner: EthAddress::deserialize(deserializer)? })
    }
}

impl FromStr for EthAddressWrapper {
    type Err = Error;

    fn from_str(s: &str) -> DevnetResult<Self> {
        Ok(EthAddressWrapper { inner: EthAddress::from_str(s)? })
    }
}

impl From<EthAddressWrapper> for Felt {
    fn from(value: EthAddressWrapper) -> Self {
        let felt: FieldElement = value.inner.into();
        let raw_felt = felt.to_bytes_be();
        Felt(raw_felt)
    }
}

impl From<EthAddressWrapper> for Felt252 {
    fn from(value: EthAddressWrapper) -> Self {
        let felt: FieldElement = value.inner.into();
        let raw_felt = felt.to_bytes_be();
        Felt252::from_bytes_be(&raw_felt)
    }
}

impl From<EthAddressWrapper> for SirAddress {
    fn from(value: EthAddressWrapper) -> Self {
        SirAddress(value.into())
    }
}
