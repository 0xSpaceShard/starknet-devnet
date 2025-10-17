// build.rs
use std::path::PathBuf;
use std::{env, fs};

use serde_json::Value;
use serde_json_canonicalizer::to_vec as canonical_to_vec;

fn compile_at_build_time(input: &Value) -> serde_json::Value {
    usc::compile_contract(input.clone())
        .unwrap_or_else(|e| panic!("usc::compile_contract failed in build.rs: {e}"))
}

#[allow(clippy::expect_used)]
fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest directory"));
    let input_dir = manifest_dir.join("precompiled").join("inputs");

    //   let fastpath_enabled = env::var("CARGO_FEATURE_FASTPATH").is_ok();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Failed to get output directory"));
    let gen_path = out_dir.join("usc_fastpath.rs");

    //   if !fastpath_enabled {
    //       // feature disabled → emit a stub
    //       fs::write(&gen_path, "pub fn lookup(_: &str) -> Option<&'static [u8]> { None }\n")
    //           .unwrap();
    //       return;
    //   }

    let mut json_files = vec![];
    if input_dir.exists() {
        for e in fs::read_dir(&input_dir).expect("Failed to read precompiled inputs directory") {
            let p = e.expect("Failed to read directory entry").path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                json_files.push(p);
            }
        }
        json_files.sort();
    }

    if json_files.is_empty() {
        fs::write(&gen_path, "pub fn lookup(_: &str) -> Option<&'static [u8]> { None }\n")
            .expect("Failed to write empty lookup function");
        println!("cargo:rerun-if-changed=precompiled/inputs");
        println!("cargo:rerun-if-changed=build.rs");
        return;
    }

    let mut code = String::from("// @generated — DO NOT EDIT\n");

    // Emit OUTPUT_* and DIGEST_* constants
    for (idx, path) in json_files.iter().enumerate() {
        let raw = fs::read(path).expect("Failed to read JSON file");
        let input_val: Value = serde_json::from_slice(&raw).expect("Failed to parse JSON file");

        let canon_bytes = canonical_to_vec(&input_val).expect("Failed to canonicalize JSON");

        // Use blake3 for hashing instead of sha256 and convert to hex without using hex crate
        let hash = blake3::hash(&canon_bytes).to_string();

        let casm_json = compile_at_build_time(&input_val);
        let casm_bytes =
            serde_json::to_vec(&casm_json).expect("Failed to serialize compiled contract to JSON");

        code.push_str(&format!("pub static OUTPUT_{idx}: &[u8] = &{:?};\n", casm_bytes));
        code.push_str(&format!("pub const DIGEST_{idx}: &str = \"{hash}\";\n"));
    }

    // Emit lookup()
    code.push_str(
        "\npub fn lookup(hash_hex: &str) -> Option<&'static [u8]> {\n    match hash_hex {\n",
    );
    for idx in 0..json_files.len() {
        code.push_str(&format!("        DIGEST_{idx} => Some(OUTPUT_{idx}),\n"));
    }
    code.push_str("        _ => None,\n    }\n}\n");

    fs::write(&gen_path, code).expect("Failed to write generated code to output file");

    println!("cargo:rerun-if-changed=precompiled/inputs");
    println!("cargo:rerun-if-changed=build.rs");
}
