use num_bigint::BigUint;
use starknet_types_core::felt::Felt;

use crate::error::{ConversionError, DevnetResult, Error};

/// Returns (high, low)
pub fn split_biguint(biguint: BigUint) -> (Felt, Felt) {
    let high = Felt::from(&biguint >> 128);
    let low_mask = (BigUint::from(1_u8) << 128) - 1_u8;
    let low = Felt::from(biguint & low_mask);
    (high, low)
}

/// Join high and low part of a felt as biguint
pub fn join_felts(high: &Felt, low: &Felt) -> BigUint {
    let high = high.to_biguint();
    let low = low.to_biguint();
    (high << 128) + low
}

pub fn felt_from_prefixed_hex(s: &str) -> DevnetResult<Felt> {
    if !s.starts_with("0x") {
        Err(Error::ConversionError(ConversionError::CustomFromHexError(format!(
            "Missing prefix 0x in {s}"
        ))))
    } else {
        Felt::from_hex(s)
            .map_err(|e| Error::ConversionError(ConversionError::CustomFromHexError(e.to_string())))
    }
}

pub fn try_felt_to_num<T: TryFrom<BigUint>>(f: Felt) -> Result<T, <T as TryFrom<BigUint>>::Error> {
    f.to_biguint().try_into()
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
