use serde_json::Value;
use serde_json_canonicalizer::to_vec as canonical_to_vec;
use sha2::{Digest, Sha256};
pub fn canonical_sha256_hex(v: &Value) -> Result<String, serde_json::Error> {
    let bytes = match canonical_to_vec(v) {
        Ok(bytes) => bytes,
        Err(_) => serde_json::to_vec(v)?,
    };

    // Compute hash
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());
    Ok(hash)
}
