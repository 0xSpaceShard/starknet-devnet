use std::fmt::LowerHex;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;

use ethers::types::U256;
use server::test_utils::assert_contains;
use starknet_core::constants::CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH;
use starknet_core::random_number_generator::generate_u32_random_number;
use starknet_core::utils::calculate_casm_hash;
use starknet_rs_accounts::{
    Account, AccountFactory, ArgentAccountFactory, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{
    BlockId, BlockTag, ContractClass, DeployAccountTransactionResult, ExecutionResult, FeeEstimate,
    Felt, FlattenedSierraClass, FunctionCall, NonZeroFelt,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_rs_signers::LocalWallet;
use starknet_types::felt::felt_from_prefixed_hex;

use super::background_devnet::BackgroundDevnet;
use super::constants::{ARGENT_ACCOUNT_CLASS_HASH, CAIRO_1_CONTRACT_PATH};

pub enum ImpersonationAction {
    ImpersonateAccount(Felt),
    StopImpersonateAccount(Felt),
    AutoImpersonate,
    StopAutoImpersonate,
}

/// dummy testing value
pub fn get_deployable_account_signer() -> LocalWallet {
    let new_account_private_key = "0xc248668388dbe9acdfa3bc734cc2d57a";
    starknet_rs_signers::LocalWallet::from(starknet_rs_signers::SigningKey::from_secret_scalar(
        felt_from_prefixed_hex(new_account_private_key).unwrap(),
    ))
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

pub type SierraWithCasmHash = (FlattenedSierraClass, Felt);

pub fn get_flattened_sierra_contract_and_casm_hash(sierra_path: &str) -> SierraWithCasmHash {
    let sierra_string = std::fs::read_to_string(sierra_path).unwrap();
    let sierra_class: SierraClass = serde_json::from_str(&sierra_string).unwrap();
    let casm_json = usc::compile_contract(serde_json::from_str(&sierra_string).unwrap()).unwrap();
    (sierra_class.flatten().unwrap(), calculate_casm_hash(casm_json).unwrap())
}

pub fn get_messaging_contract_in_sierra_and_compiled_class_hash() -> SierraWithCasmHash {
    let sierra_path = "../../contracts/l1-l2-artifacts/cairo_l1_l2.contract_class.sierra";
    get_flattened_sierra_contract_and_casm_hash(sierra_path)
}

pub fn get_messaging_lib_in_sierra_and_compiled_class_hash() -> SierraWithCasmHash {
    let sierra_path = "../../contracts/l1-l2-artifacts/cairo_l1_l2_lib.contract_class.sierra";
    get_flattened_sierra_contract_and_casm_hash(sierra_path)
}

pub fn get_events_contract_in_sierra_and_compiled_class_hash() -> SierraWithCasmHash {
    let events_sierra_path =
        "../../contracts/test_artifacts/cairo1/events/events_2.0.1_compiler.sierra";
    get_flattened_sierra_contract_and_casm_hash(events_sierra_path)
}

pub fn get_block_reader_contract_in_sierra_and_compiled_class_hash() -> SierraWithCasmHash {
    let timestamp_sierra_path =
        "../../contracts/test_artifacts/cairo1/block_reader/block_reader.sierra";
    get_flattened_sierra_contract_and_casm_hash(timestamp_sierra_path)
}

pub fn get_simple_contract_in_sierra_and_compiled_class_hash() -> SierraWithCasmHash {
    get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH)
}

pub async fn assert_tx_successful<T: Provider>(tx_hash: &Felt, client: &T) {
    let receipt = client.get_transaction_receipt(tx_hash).await.unwrap().receipt;
    match receipt.execution_result() {
        ExecutionResult::Succeeded => (),
        other => panic!("Should have succeeded; got: {other:?}"),
    }

    match receipt.finality_status() {
        starknet_rs_core::types::TransactionFinalityStatus::AcceptedOnL2 => (),
        other => panic!("Should have been accepted on L2; got: {other:?}"),
    }
}

pub async fn get_contract_balance(devnet: &BackgroundDevnet, contract_address: Felt) -> Felt {
    get_contract_balance_by_block_id(devnet, contract_address, BlockId::Tag(BlockTag::Latest)).await
}

pub async fn get_contract_balance_by_block_id(
    devnet: &BackgroundDevnet,
    contract_address: Felt,
    block_id: BlockId,
) -> Felt {
    let contract_call = FunctionCall {
        contract_address,
        entry_point_selector: get_selector_from_name("get_balance").unwrap(),
        calldata: vec![],
    };
    match devnet.json_rpc_client.call(contract_call, block_id).await {
        Ok(res) => {
            assert_eq!(res.len(), 1);
            res[0]
        }
        Err(e) => panic!("Call failed: {e}"),
    }
}

pub async fn assert_tx_reverted<T: Provider>(
    tx_hash: &Felt,
    client: &T,
    expected_failure_reasons: &[&str],
) {
    let receipt = client.get_transaction_receipt(tx_hash).await.unwrap().receipt;
    match receipt.execution_result() {
        ExecutionResult::Reverted { reason } => {
            for expected_reason in expected_failure_reasons {
                assert_contains(reason, expected_reason);
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

pub async fn send_ctrl_c_signal_and_wait(process: &Child) {
    send_ctrl_c_signal(process).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

async fn send_ctrl_c_signal(process: &Child) {
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

fn take_abi_from_json(value: &mut serde_json::Value) -> Result<serde_json::Value, anyhow::Error> {
    let abi_jsonified = value["abi"].take();
    assert_ne!(abi_jsonified, serde_json::json!(null));
    Ok(serde_json::from_str(abi_jsonified.as_str().unwrap())?)
}

/// Handles differences in abi serialization (some might contain spaces between properties, some
/// not) Comparing the ABIs separately as JSON-parsed values.
pub fn assert_cairo1_classes_equal(
    class_a: &ContractClass,
    class_b: &ContractClass,
) -> Result<(), anyhow::Error> {
    let mut class_a_jsonified = serde_json::to_value(class_a)?;
    let mut class_b_jsonified = serde_json::to_value(class_b)?;

    let abi_a = take_abi_from_json(&mut class_a_jsonified)?;
    let abi_b = take_abi_from_json(&mut class_b_jsonified)?;

    assert_eq!(class_a_jsonified, class_b_jsonified);
    assert_eq!(abi_a, abi_b);

    Ok(())
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
        // generate two random numbers to increase uniqueness
        let rand = format!("{}{}", generate_u32_random_number(), generate_u32_random_number());
        Self { path: format!("{name_base}-{}", rand) }
    }
}

impl Drop for UniqueAutoDeletableFile {
    fn drop(&mut self) {
        remove_file(&self.path)
    }
}

/// Declares and deploys a Cairo 1 contract; returns class hash and contract address
pub async fn declare_v3_deploy_v3(
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_class: FlattenedSierraClass,
    casm_hash: Felt,
    ctor_args: &[Felt],
) -> Result<(Felt, Felt), anyhow::Error> {
    let salt = Felt::ZERO;
    let declaration_result = account.declare_v3(Arc::new(contract_class), casm_hash).send().await?;

    // deploy the contract
    let contract_factory = ContractFactory::new(declaration_result.class_hash, account);
    contract_factory.deploy_v3(ctor_args.to_vec(), salt, false).send().await?;

    // generate the address of the newly deployed contract
    let contract_address = get_udc_deployed_address(
        salt,
        declaration_result.class_hash,
        &starknet_rs_core::utils::UdcUniqueness::NotUnique,
        ctor_args,
    );

    Ok((declaration_result.class_hash, contract_address))
}

/// Assumes the Cairo1 OpenZepplin contract is declared in the target network.
pub async fn deploy_oz_account(
    devnet: &BackgroundDevnet,
) -> Result<(DeployAccountTransactionResult, LocalWallet), anyhow::Error> {
    let signer = get_deployable_account_signer();
    let salt = Felt::THREE;
    let factory = OpenZeppelinAccountFactory::new(
        felt_from_prefixed_hex(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)?,
        devnet.json_rpc_client.chain_id().await?,
        signer.clone(),
        devnet.clone_provider(),
    )
    .await?;

    let deployment = factory.deploy_v1(salt);

    let account_address = deployment.address();
    devnet.mint(account_address, 1e18 as u128).await;
    let deployment_result = deployment.send().await?;

    Ok((deployment_result, signer))
}

/// Assumes the Argent account contract is declared in the target network.
pub async fn deploy_argent_account(
    devnet: &BackgroundDevnet,
) -> Result<(DeployAccountTransactionResult, LocalWallet), anyhow::Error> {
    let signer = get_deployable_account_signer();
    let salt = Felt::THREE;
    let factory = ArgentAccountFactory::new(
        felt_from_prefixed_hex(ARGENT_ACCOUNT_CLASS_HASH)?,
        devnet.json_rpc_client.chain_id().await?,
        Felt::ZERO,
        signer.clone(),
        devnet.clone_provider(),
    )
    .await?;

    let deployment = factory.deploy_v1(salt);

    let account_address = deployment.address();
    devnet.mint(account_address, 1e18 as u128).await;
    let deployment_result = deployment.send().await?;

    Ok((deployment_result, signer))
}

/// Assert that the set of elements of `iterable1` is a subset of the elements of `iterable2` and
/// vice versa.
pub fn assert_equal_elements<T>(iterable1: &[T], iterable2: &[T])
where
    T: PartialEq,
{
    assert_eq!(iterable1.len(), iterable2.len());
    for e in iterable1 {
        assert!(iterable2.contains(e));
    }
}

pub fn felt_to_u256(f: Felt) -> U256 {
    U256::from_big_endian(&f.to_bytes_be())
}

pub fn get_gas_units_and_gas_price(fee_estimate: FeeEstimate) -> (u64, u128) {
    let gas_price =
        u128::from_le_bytes(fee_estimate.gas_price.to_bytes_le()[0..16].try_into().unwrap());
    let gas_units = fee_estimate
        .overall_fee
        .field_div(&NonZeroFelt::from_felt_unchecked(fee_estimate.gas_price));

    (gas_units.to_le_digits().first().cloned().unwrap(), gas_price)
}

#[cfg(test)]
mod test_unique_auto_deletable_file {
    use std::path::Path;

    use crate::common::utils::UniqueAutoDeletableFile;

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
