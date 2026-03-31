use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Proof represented as a vector of u32 values.
/// When serialized, it's encoded as a base64 string for compact representation.
/// When deserialized, it accepts both base64 strings and arrays of u32 values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof(pub Vec<u32>);

impl Proof {
    pub fn new(data: Vec<u32>) -> Self {
        Self(data)
    }

    pub fn inner(&self) -> &Vec<u32> {
        &self.0
    }

    pub fn into_inner(self) -> Vec<u32> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u32>> for Proof {
    fn from(data: Vec<u32>) -> Self {
        Self(data)
    }
}

impl From<Proof> for Vec<u32> {
    fn from(proof: Proof) -> Self {
        proof.0
    }
}

impl AsRef<[u32]> for Proof {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

impl Serialize for Proof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Vec<u32> to bytes (little-endian)
        let bytes: Vec<u8> = self.0.iter().flat_map(|&val| val.to_le_bytes()).collect();
        let base64_string = base64::engine::general_purpose::STANDARD.encode(&bytes);
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
            Array(Vec<u32>),
        }

        match ProofFormat::deserialize(deserializer)? {
            ProofFormat::Base64(s) => {
                // Decode base64 string to bytes
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&s)
                    .map_err(|e| D::Error::custom(format!("Invalid base64: {}", e)))?;

                // Convert bytes back to Vec<u32> (little-endian)
                if bytes.len() % 4 != 0 {
                    return Err(D::Error::custom(format!(
                        "Invalid proof length: {} bytes (must be multiple of 4)",
                        bytes.len()
                    )));
                }

                let u32_vec = bytes
                    .chunks_exact(4)
                    .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();

                Ok(Proof(u32_vec))
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
        let proof = Proof(vec![0x12345678, 0xABCDEF00, 0x11223344, 0x55667788]);
        let serialized = serde_json::to_string(&proof).unwrap();

        // Should be a base64 string, not an array
        assert!(serialized.starts_with('"'));
        assert!(serialized.ends_with('"'));
        assert!(!serialized.contains('['));
    }

    #[test]
    fn test_deserialize_proof_from_base64() {
        let original = Proof(vec![0x12345678, 0xABCDEF00, 0x11223344, 0x55667788]);
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Proof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_deserialize_proof_from_array() {
        let json = "[305419896,2882400000,287454020,1432778632]";
        let proof: Proof = serde_json::from_str(json).unwrap();

        assert_eq!(proof.0, vec![0x12345678, 0xABCDEF00, 0x11223344, 0x55667788]);
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
    fn test_invalid_length() {
        // Create a base64 string with length not divisible by 4
        let bytes = vec![1, 2, 3, 4, 5]; // 5 bytes, not divisible by 4
        let base64_string = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let json = format!("\"{}\"", base64_string);
        let result = serde_json::from_str::<Proof>(&json);

        assert!(result.is_err());
    }
}
