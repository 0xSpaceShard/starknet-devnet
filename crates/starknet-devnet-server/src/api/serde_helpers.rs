/// A module that deserializes `[]` and `{}` optionally
pub mod empty_params {
    use serde::de::Error as DeError;
    use serde::{Deserialize, Deserializer};
    use serde_json::Value;

    pub fn deserialize<'de, D>(d: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(d)?;

        match value {
            Value::Null => Ok(()),
            Value::Object(obj) if obj.is_empty() => Ok(()),
            Value::Array(arr) if arr.is_empty() => Ok(()),
            other => Err(DeError::custom(format!(
                "expected empty object or array for params; got: {other:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::de::IntoDeserializer;
    use serde_json::{self, Value};

    use super::empty_params::deserialize;

    fn test_deserialization(json_str: &str) -> Result<(), serde_json::Error> {
        let value: Value = serde_json::from_str(json_str)?;
        let deserializer = value.into_deserializer();
        deserialize(deserializer)
    }

    #[test]
    fn deserialize_empty_object() {
        let json_str = "{}";
        assert!(test_deserialization(json_str).is_ok());
    }

    #[test]
    fn deserialize_empty_array() {
        let json_str = "[]";
        assert!(test_deserialization(json_str).is_ok());
    }

    #[test]
    fn deserialize_non_empty_object() {
        let json_str = "{\"key\": \"value\"}";
        assert!(test_deserialization(json_str).is_err());
    }

    #[test]
    fn deserialize_non_empty_array() {
        let json_str = "[1, 2, 3]";
        assert!(test_deserialization(json_str).is_err());
    }

    #[test]
    fn deserialize_other_types() {
        let json_str = "\"string\"";
        assert!(test_deserialization(json_str).is_err());
    }

    #[test]
    fn deserialize_null() {
        let value: Value = serde_json::Value::Null;
        let deserializer = value.into_deserializer();
        assert!(deserialize(deserializer).is_ok());
    }
}
