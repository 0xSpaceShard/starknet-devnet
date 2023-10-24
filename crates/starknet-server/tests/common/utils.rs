use std::fmt::LowerHex;
use std::fs;
use std::path::Path;

use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use hyper::{Body, Response};
use starknet_in_rust::core::contract_address::compute_casm_class_hash;
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{ExecutionResult, FieldElement, FlattenedSierraClass};
use starknet_rs_providers::Provider;
use starknet_rs_signers::LocalWallet;
use starknet_types::felt::Felt;

pub async fn get_json_body(resp: Response<Body>) -> serde_json::Value {
    let resp_body = resp.into_body();
    let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
    serde_json::from_slice(&resp_body_bytes).unwrap()
}

/// dummy testing value
pub fn get_deployable_account_signer() -> LocalWallet {
    let new_account_private_key = "0xc248668388dbe9acdfa3bc734cc2d57a";
    starknet_rs_signers::LocalWallet::from(starknet_rs_signers::SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(new_account_private_key).unwrap(),
    ))
}

/// resolve a path relative to the current directory (starknet-server)
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

    let contract_class: cairo_lang_starknet::contract_class::ContractClass =
        serde_json::from_str(&sierra_artifact).unwrap();

    let casm_contract_class =
        CasmContractClass::from_contract_class(contract_class, false).unwrap();
    let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

    (sierra_class.flatten().unwrap(), Felt::from(compiled_class_hash).into())
}

pub async fn assert_tx_successful<T: Provider>(tx_hash: &FieldElement, client: &T) {
    let receipt = client.get_transaction_receipt(tx_hash).await.unwrap();
    match receipt.execution_result() {
        ExecutionResult::Succeeded => (),
        other => panic!("Should have succeeded; got: {other:?}"),
    }
}

pub async fn assert_tx_reverted<T: Provider>(
    tx_hash: &FieldElement,
    client: &T,
    expected_failure_reasons: &[&str],
) {
    let receipt = client.get_transaction_receipt(tx_hash).await.unwrap();
    match receipt.execution_result() {
        ExecutionResult::Reverted { reason } => {
            for expected_reason in expected_failure_reasons {
                reason.contains(expected_reason);
            }
        }
        other => panic!("Should have reverted; got: {other:?}; receipt: {receipt:?}"),
    }
}

pub fn to_hex_felt<T: LowerHex>(value: &T) -> String {
    format!("{value:#x}")
}

pub fn to_num_as_hex<T: LowerHex>(value: &T) -> String {
    format!("{value:#x}")
}

pub fn iter_to_hex_felt<T: LowerHex>(iterable: &[T]) -> Vec<String> {
    iterable.iter().map(to_hex_felt).collect()
}
