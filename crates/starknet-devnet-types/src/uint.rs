use primitive_types::U256;
use serde::Deserialize;

pub type Balance = U256;

pub fn u256_deserialize_from_dec_string<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let number = serde_json::Number::deserialize(deserializer)?;
    let s = number.to_string();
    U256::from_dec_str(&s).map_err(serde::de::Error::custom)
}
