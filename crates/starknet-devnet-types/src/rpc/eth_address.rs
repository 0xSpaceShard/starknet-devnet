use std::str::FromStr;

use starknet_api::core::{EthAddress as ApiEthAddress, L1Address};
use starknet_rs_core::types::{EthAddress, Felt};

use crate::error::{ConversionError, DevnetResult, Error};
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
        value.inner.into()
    }
}

impl From<ApiEthAddress> for EthAddressWrapper {
    fn from(value: ApiEthAddress) -> Self {
        EthAddressWrapper { inner: EthAddress::from_bytes(value.0.to_fixed_bytes()) }
    }
}

impl TryFrom<L1Address> for EthAddressWrapper {
    type Error = Error;

    fn try_from(value: L1Address) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: EthAddress::from_felt(&value.0).map_err(|e| {
                Error::ConversionError(ConversionError::OutOfRangeError(e.to_string()))
            })?,
        })
    }
}
