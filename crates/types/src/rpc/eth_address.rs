use std::str::FromStr;

use cairo_felt::Felt252;
use starknet_api::core::EthAddress as ApiEthAddress;
use starknet_rs_core::types::EthAddress;
use starknet_rs_ff::FieldElement;

use crate::error::{DevnetResult, Error};
use crate::felt::Felt;
use crate::{impl_wrapper_deserialize, impl_wrapper_serialize};

#[derive(Debug, Clone, Eq, PartialEq)]
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

impl From<ApiEthAddress> for EthAddressWrapper {
    fn from(value: ApiEthAddress) -> Self {
        // Can be simplified if https://github.com/xJonathanLEI/starknet-rs/pull/506 is merged.
        let eth_address = format!("{:?}", value.0);
        let eth_address = eth_address.strip_prefix("0x").unwrap_or(&eth_address);
        let eth_address: EthAddress = EthAddress::from_hex(eth_address)
            .expect("EthAddress from starknet_api is out of range");
        EthAddressWrapper { inner: eth_address }
    }
}

impl From<EthAddressWrapper> for Felt252 {
    fn from(value: EthAddressWrapper) -> Self {
        let felt: FieldElement = value.inner.into();
        let raw_felt = felt.to_bytes_be();
        Felt252::from_bytes_be(&raw_felt)
    }
}
