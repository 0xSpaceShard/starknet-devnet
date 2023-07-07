use crate::felt::Felt;
use crate::DevnetResult;

pub trait ToHexString {
    fn to_prefixed_hex_str(&self) -> String;
    fn to_nonprefixed_hex_str(&self) -> String;
}

pub trait HashProducer {
    fn generate_hash(&self) -> DevnetResult<Felt>;
}
