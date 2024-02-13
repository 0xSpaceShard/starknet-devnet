/// A module that deserializes `[]` optionally
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
            Value::Object(obj) if obj.is_empty() => Ok(()),
            Value::Array(arr) if arr.is_empty() => Ok(()),
            other => Err(DeError::custom(format!(
                "expected empty object or array for params; got: {other:?}"
            ))),
        }
    }
}
