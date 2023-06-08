use crate::felt::Felt;

pub(crate) fn dummy_felt() -> Felt {
    Felt::from_prefixed_hex_str("0xF9").unwrap()
}
