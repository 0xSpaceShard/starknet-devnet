use crate::felt::Felt;

pub trait ToHexString {
    fn to_prefixed_hex_str(&self) -> String;
    fn to_nonprefixed_hex_str(&self) -> String;
}

pub trait HashProducer {
    type Error;
    fn generate_hash(&self) -> Result<Felt, Self::Error>;
}

pub trait ToDecimalString {
    fn to_decimal_string(&self) -> String;
}
