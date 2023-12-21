use std::fmt::LowerHex;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};

use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use hyper::{Body, Response};
use random_number_generator::generate_u32_random_number;
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{ExecutionResult, FieldElement, FlattenedSierraClass};
use starknet_rs_providers::Provider;
use starknet_rs_signers::LocalWallet;
use starknet_types::contract_class::compute_casm_class_hash;

pub async fn get_json_body(resp: Response<Body>) -> serde_json::Value {
    let resp_body = resp.into_body();
    let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
    serde_json::from_slice(&resp_body_bytes).unwrap()
}

pub async fn get_string_body(resp: Response<Body>) -> String {
    let resp_body = resp.into_body();
    let body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
    String::from_utf8(body_bytes.to_vec()).unwrap()
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

pub fn get_flattened_sierra_contract_and_casm_hash(
    sierra_path: &str,
) -> (FlattenedSierraClass, FieldElement) {
    let sierra_string = std::fs::read_to_string(sierra_path).unwrap();
    let sierra_class: SierraClass = serde_json::from_str(&sierra_string).unwrap();
    let contract_class: cairo_lang_starknet::contract_class::ContractClass =
        serde_json::from_str(&sierra_string).unwrap();

    let casm_contract_class =
        CasmContractClass::from_contract_class(contract_class, false).unwrap();
    let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();
    (sierra_class.flatten().unwrap(), compiled_class_hash.into())
}

pub fn get_messaging_contract_in_sierra_and_compiled_class_hash()
-> (FlattenedSierraClass, FieldElement) {
    let sierra_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/cairo1/messaging/cairo_1_l1l2.sierra");
    get_flattened_sierra_contract_and_casm_hash(sierra_path)
}

pub fn get_messaging_lib_in_sierra_and_compiled_class_hash() -> (FlattenedSierraClass, FieldElement)
{
    let sierra_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/cairo1/messaging/cairo_1_l1l2_lib.sierra");
    get_flattened_sierra_contract_and_casm_hash(sierra_path)
}

pub fn get_events_contract_in_sierra_and_compiled_class_hash()
-> (FlattenedSierraClass, FieldElement) {
    let events_sierra_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/cairo1/events/events_2.0.1_compiler.sierra"
    );
    get_flattened_sierra_contract_and_casm_hash(events_sierra_path)
}

pub fn get_timestamp_contract_in_sierra_and_compiled_class_hash()
-> (FlattenedSierraClass, FieldElement) {
    let timestamp_sierra_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/cairo1/timestamp/timestamp_v2.3.1_compiler.sierra"
    );
    get_flattened_sierra_contract_and_casm_hash(timestamp_sierra_path)
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

pub fn get_unix_timestamp_as_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("should get current UNIX timestamp")
        .as_secs()
}

pub async fn send_ctrl_c_signal(process: &Child) {
    #[cfg(windows)]
    {
        // To send SIGINT signal on windows, windows-kill is needed
        let mut kill = Command::new("windows-kill")
            .args(["-SIGINT", process.id().to_string().as_str()])
            .spawn()
            .unwrap();
        kill.wait().unwrap();
    }

    #[cfg(unix)]
    {
        let mut kill = Command::new("kill")
            .args(["-s", "SIGINT", process.id().to_string().as_str()])
            .spawn()
            .unwrap();
        kill.wait().unwrap();
    }
}

/// Wrapper of file name which attempts to delete the file when the variable is dropped.
/// Appends a random sequence to the file name base to make it unique.
/// Prevents name collisions - no need to come up with unique names for files (e.g. when dumping).
/// Automatically deletes the underlying file when the variable is dropped - no need to remember
/// deleting.
pub struct UniqueAutoDeletableFile {
    pub path: String,
}

impl UniqueAutoDeletableFile {
    /// Appends a random sequence to the name_base to make it unique
    /// Unlike [NamedTempFile](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html),
    /// it doesn't create the file.
    pub fn new(name_base: &str) -> Self {
        Self { path: format!("{name_base}-{}", generate_u32_random_number()) }
    }
}

impl Drop for UniqueAutoDeletableFile {
    fn drop(&mut self) {
        remove_file(&self.path)
    }
}

#[cfg(test)]
mod test_unique_auto_deletable_file {
    use std::path::Path;

    use super::UniqueAutoDeletableFile;

    #[test]
    fn test_deleted() {
        let file = UniqueAutoDeletableFile::new("foo");
        let saved_file_path = file.path.clone();
        assert!(!Path::new(&file.path).exists());

        std::fs::File::create(&file.path).unwrap();
        assert!(Path::new(&file.path).exists());

        drop(file);
        assert!(!Path::new(&saved_file_path).exists());
    }

    #[test]
    fn test_dropping_successful_if_file_not_created() {
        let file = UniqueAutoDeletableFile::new("foo");
        drop(file);
        // if everything ok, the test should just exit successfully
    }

    #[test]
    fn test_file_names_unique() {
        let common_prefix = "foo";
        // run it many times to increase the probability of being secure
        for _ in 0..1_000_000 {
            let file1 = UniqueAutoDeletableFile::new(common_prefix);
            let file2 = UniqueAutoDeletableFile::new(common_prefix);
            assert_ne!(file1.path, file2.path);
        }
    }
}
