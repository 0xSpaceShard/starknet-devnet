use std::str::FromStr;

use serde::de::{Error as DeError, Visitor};
use serde::Deserializer;
use serde_with::DeserializeAs;

/// A module that deserializes `[]` optionally
pub mod empty_params {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(d: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        let seq = Option::<Vec<()>>::deserialize(d)?.unwrap_or_default();
        if !seq.is_empty() {
            return Err(serde::de::Error::custom(format!(
                "expected params sequence with length 0 but got {}",
                seq.len()
            )));
        }
        Ok(())
    }
}

/// Implement `serde` custom deserialization for `u128`.
/// To ensure that we can accept hexadecimal string, this
/// custom deserializer for `u128` ensures we also stay in the
/// range of `u128` values.
pub struct U128HexOrDec;
/// The related Visitor struct ensure we can parse `u128` value of two forms:
/// `"1234"` which is a decimal string.
/// `"0xff"` which is a hexadecimal string.
pub struct U128HexOrDecVisitor;

impl<'de> DeserializeAs<'de, u128> for U128HexOrDec {
    fn deserialize_as<D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(U128HexOrDecVisitor)
    }
}

impl<'de> Visitor<'de> for U128HexOrDecVisitor {
    type Value = u128;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a u128 decimal or hexadecimal string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if value.starts_with("0x") {
            u128::from_str_radix(&value[2..], 16)
                .map_err(|e| DeError::custom(format!("invalid u128 / hex string: {e}")))
        } else if let Ok(num) = u128::from_str(value) {
            Ok(num)
        } else {
            Err(DeError::custom("Expecting a u128 decimal string or a hexadecimal string."))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_with::serde_as;

    use super::*;

    #[serde_as]
    #[derive(Deserialize)]
    struct TestStruct(#[serde_as(as = "U128HexOrDec")] pub u128);

    #[test]
    fn u128_hexadecimal_ok() {
        let r = serde_json::from_str::<TestStruct>("\"0xff\"").unwrap();
        assert_eq!(r.0, 0xff_u128);
    }

    #[test]
    fn u128_hexadecimal_out_of_range() {
        match serde_json::from_str::<TestStruct>("\"0x01ffffffffffffffffffffffffffffffff\"") {
            Ok(_) => panic!("Expecting deserialization error"),
            Err(e) => {
                assert_eq!(
                    e.to_string(),
                    "invalid u128 / hex string: number too large to fit in target type at line 1 \
                     column 38"
                );
            }
        }
    }

    #[test]
    fn u128_number_ok() {
        let r = serde_json::from_str::<TestStruct>("\"1234\"").unwrap();
        assert_eq!(r.0, 1234);
    }
}
