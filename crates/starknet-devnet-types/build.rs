// build.rs
use std::path::PathBuf;
use std::{env, fs};

use serde_json::Value;
use sha2::{Digest, Sha256};

fn compile_at_build_time(input: &Value) -> serde_json::Value {
    usc::compile_contract(input.clone())
        .unwrap_or_else(|e| panic!("usc::compile_contract failed in build.rs: {e}"))
}

fn canonicalize(v: &Value) -> Value {
    use std::collections::BTreeMap;

    use serde_json::{Map, Value};
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

#[allow(clippy::unwrap_used)]
fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let input_dir = manifest_dir.join("precompiled").join("inputs");

    //   let fastpath_enabled = env::var("CARGO_FEATURE_FASTPATH").is_ok();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let gen_path = out_dir.join("usc_fastpath.rs");

    //   if !fastpath_enabled {
    //       // feature disabled → emit a stub
    //       fs::write(&gen_path, "pub fn lookup(_: &str) -> Option<&'static [u8]> { None }\n")
    //           .unwrap();
    //       return;
    //   }

    let mut json_files = vec![];
    if input_dir.exists() {
        for e in fs::read_dir(&input_dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                json_files.push(p);
            }
        }
        json_files.sort();
    }

    if json_files.is_empty() {
        fs::write(&gen_path, "pub fn lookup(_: &str) -> Option<&'static [u8]> { None }\n").unwrap();
        println!("cargo:rerun-if-changed=fastpath/inputs");
        println!("cargo:rerun-if-changed=build.rs");
        return;
    }

    let mut code = String::from("// @generated — DO NOT EDIT\n");

    // Emit OUTPUT_* and DIGEST_* constants
    for (idx, path) in json_files.iter().enumerate() {
        let raw = fs::read(path).unwrap();
        let input_val: Value = serde_json::from_slice(&raw).unwrap();

        let canon = canonicalize(&input_val);
        let canon_bytes = serde_json::to_vec(&canon).unwrap();

        let mut h = Sha256::new();
        h.update(&canon_bytes);
        let digest_hex = hex::encode(h.finalize());

        let casm_json = compile_at_build_time(&input_val);
        let casm_bytes = serde_json::to_vec(&casm_json).unwrap();

        code.push_str(&format!("pub static OUTPUT_{idx}: &[u8] = &{:?};\n", casm_bytes));
        code.push_str(&format!("pub const DIGEST_{idx}: &str = \"{digest_hex}\";\n"));
    }

    // Emit lookup()
    code.push_str(
        "\npub fn lookup(hash_hex: &str) -> Option<&'static [u8]> {\n    match hash_hex {\n",
    );
    for idx in 0..json_files.len() {
        code.push_str(&format!("        DIGEST_{idx} => Some(OUTPUT_{idx}),\n"));
    }
    code.push_str("        _ => None,\n    }\n}\n");

    fs::write(&gen_path, code).unwrap();

    println!("cargo:rerun-if-changed=fastpath/inputs");
    println!("cargo:rerun-if-changed=build.rs");
}
