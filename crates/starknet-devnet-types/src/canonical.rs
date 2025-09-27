use std::collections::BTreeMap;

use serde_json::Value;

pub fn canonicalize(v: &Value) -> Value {
    use serde_json::Map;
    match v {
        Value::Object(m) => {
            let mut bm = BTreeMap::new();
            for (k, vv) in m {
                bm.insert(k.clone(), canonicalize(vv));
            }
            Value::Object(Map::from_iter(bm))
        }
        Value::Array(a) => Value::Array(a.iter().map(canonicalize).collect()),
        _ => v.clone(),
    }
}

pub fn canonical_sha256_hex(v: &Value) -> Result<String, serde_json::Error> {
    use sha2::{Digest, Sha256};
    let canon = canonicalize(v);
    let bytes = serde_json::to_vec(&canon)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(hex::encode(h.finalize()))
}
