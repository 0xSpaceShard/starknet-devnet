use std::str::FromStr;

use starknet_in_rust::felt::Felt252;
use starknet_in_rust::utils::Address as SirAddress;
use starknet_rs_core::types::EthAddress;
use starknet_rs_ff::FieldElement;

use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use crate::{impl_wrapper_deserialize, impl_wrapper_serialize};

#[derive(Debug, Clone)]
pub struct EthAddressWrapper {
    pub inner: EthAddress,
}

impl_wrapper_serialize!(EthAddressWrapper);
impl_wrapper_deserialize!(EthAddressWrapper, EthAddress);

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
