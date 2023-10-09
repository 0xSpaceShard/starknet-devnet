use std::fs;
use std::path::Path;

use hyper::{Body, Response};
use starknet_in_rust::core::contract_address::compute_casm_class_hash;
use starknet_in_rust::CasmContractClass;
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{FieldElement, FlattenedSierraClass};
use starknet_rs_signers::{LocalWallet, SigningKey};
use starknet_types::felt::Felt;

use super::constants::{PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_PRIVATE_KEY};

pub async fn get_json_body(resp: Response<Body>) -> serde_json::Value {
    let resp_body = resp.into_body();
    let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
    serde_json::from_slice(&resp_body_bytes).unwrap()
}

/// Assumes Devnet has been run with the usual account seed and returns
/// the signer and address of the 0th account
pub fn get_predeployed_account_props() -> (LocalWallet, FieldElement) {
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_PRIVATE_KEY).unwrap(),
    ));
    let address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
    (signer, address)
}

/// dummy testing value
pub fn get_deployable_account_signer() -> LocalWallet {
    let new_account_private_key = "0xc248668388dbe9acdfa3bc734cc2d57a";
    starknet_rs_signers::LocalWallet::from(starknet_rs_signers::SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(new_account_private_key).unwrap(),
    ))
}

/// resolve a path relative to the crates directory
pub fn resolve_path(relative_path: &str) -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{manifest_dir}/{relative_path}")
}

pub fn remove_file(path: &str) {
    let file_path = Path::new(path);
    if file_path.exists() {
        fs::remove_file(file_path).expect("Could not remove file");
    }
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let reader = std::fs::File::open(path).unwrap();
    let loaded: T = serde_json::from_reader(reader).unwrap();
    loaded
}

pub fn get_events_contract_in_sierra_and_compiled_class_hash()
-> (FlattenedSierraClass, FieldElement) {
    let sierra_artifact = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/cairo1/events/events_2.0.1_compiler.sierra"
    ))
    .unwrap();
    let sierra_class: SierraClass = serde_json::from_str(&sierra_artifact).unwrap();

    let contract_class: starknet_in_rust::ContractClass =
        serde_json::from_str(&sierra_artifact).unwrap();

    let casm_contract_class =
        CasmContractClass::from_contract_class(contract_class, false).unwrap();
    let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

    (sierra_class.flatten().unwrap(), Felt::from(compiled_class_hash).into())
}
