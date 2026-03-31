use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Proof represented as a vector of u8 values.
/// When serialized, it's encoded as a base64 string for compact representation.
/// When deserialized, it accepts both base64 strings and arrays of u8 values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof(pub Vec<u8>);

impl Proof {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn inner(&self) -> &Vec<u8> {
        &self.0
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u8>> for Proof {
    fn from(data: Vec<u8>) -> Self {
        Self(data)
    }
}

impl From<Proof> for Vec<u8> {
    fn from(proof: Proof) -> Self {
        proof.0
    }
}

impl AsRef<[u8]> for Proof {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Proof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let base64_string = base64::engine::general_purpose::STANDARD.encode(&self.0);
        serializer.serialize_str(&base64_string)
    }
}

impl<'de> Deserialize<'de> for Proof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ProofFormat {
            Base64(String),
            Array(Vec<u8>),
        }

        match ProofFormat::deserialize(deserializer)? {
            ProofFormat::Base64(s) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&s)
                    .map_err(|e| D::Error::custom(format!("Invalid base64: {}", e)))?;

                Ok(Proof(bytes))
            }
            ProofFormat::Array(vec) => Ok(Proof(vec)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_serialize_proof_as_base64() {
        let proof = Proof(vec![0x12, 0xAB, 0x44, 0x88]);
        let serialized = serde_json::to_string(&proof).unwrap();

        // Should be a base64 string, not an array
        assert!(serialized.starts_with('"'));
        assert!(serialized.ends_with('"'));
        assert!(!serialized.contains('['));
    }

    #[test]
    fn test_deserialize_proof_from_base64() {
        let original = Proof(vec![0x12, 0xAB, 0x44, 0x88]);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Proof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_deserialize_proof_from_array() {
        let json = "[18,171,68,136]";
        let proof: Proof = serde_json::from_str(json).unwrap();

        assert_eq!(proof.0, vec![0x12, 0xAB, 0x44, 0x88]);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = Proof(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Proof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_empty_proof() {
        let proof = Proof(vec![]);
        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: Proof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(proof, deserialized);
        assert!(deserialized.0.is_empty());
    }

    #[test]
    fn test_invalid_base64() {
        let json = "\"not valid base64!!!\"";
        let result = serde_json::from_str::<Proof>(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_any_length_base64_is_valid() {
        // Any byte length is valid for Vec<u8>
        let bytes = vec![1, 2, 3, 4, 5];
        let base64_string = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let json = format!("\"{}\"", base64_string);
        let result: Proof = serde_json::from_str(&json).unwrap();

        assert_eq!(result.0, bytes);
    }
}
