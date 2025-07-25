use std::sync::Arc;
use std::time;

use serde_json::json;
use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, Felt, FunctionCall, StarknetError, TransactionExecutionStatus,
    TransactionStatus,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::{
    UniqueAutoDeletableFile, assert_contains, declare_v3_deploy_v3, extract_message_error,
    extract_nested_error, get_block_reader_contract_artifacts,
    get_timestamp_asserter_contract_artifacts, get_unix_timestamp_as_seconds, increase_time,
    send_ctrl_c_signal_and_wait, set_time,
};

const DUMMY_ADDRESS: u128 = 1;
const DUMMY_AMOUNT: u128 = 1;
// buffer should be always lower than the time change that we are testing
const BUFFER_TIME_SECONDS: u64 = 30;

async fn sleep_until_new_timestamp() {
    // Sometimes sleeping for 1 second isn't enough.
    tokio::time::sleep(time::Duration::from_millis(1500)).await
}

pub fn assert_ge_with_buffer(val1: u64, val2: u64) {
    assert!(val1 >= val2, "Failed inequation: {val1:?} >= {val2:?}");
    let upper_limit = val2 + BUFFER_TIME_SECONDS;
    assert!(val1 <= upper_limit, "Failed inequation: {val1:?} <= {upper_limit:?}");
}

pub fn assert_gt_with_buffer(val1: u64, val2: u64) {
    assert!(val1 > val2, "Failed inequation: {val1:?} > {val2:?}");
    let upper_limit = val2 + BUFFER_TIME_SECONDS;
    assert!(val1 <= upper_limit, "Failed inequation: {val1:?} <= {upper_limit:?}");
}

fn assert_eq_with_buffer(val1: u64, val2: u64) {
    assert!(
        val1.abs_diff(val2) < BUFFER_TIME_SECONDS,
        "Failed equality assertion with buffer: {val1} != {val2}"
    );
}

pub async fn setup_timestamp_contract(devnet: &BackgroundDevnet) -> Felt {
    let (signer, address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    // declare
    let (cairo_1_contract, casm_class_hash) = get_block_reader_contract_artifacts();
    let declaration_result = predeployed_account
        .declare_v3(Arc::new(cairo_1_contract), casm_class_hash)
        .send()
        .await
        .unwrap();
    let predeployed_account = Arc::new(predeployed_account);

    // deploy
    let contract_factory =
        ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
    contract_factory.deploy_v3(vec![], Felt::ZERO, false).send().await.unwrap();

    get_udc_deployed_address(
        Felt::ZERO,
        declaration_result.class_hash,
        &starknet_rs_core::utils::UdcUniqueness::NotUnique,
        &[],
    )
}

pub async fn get_current_timestamp(
    devnet: &BackgroundDevnet,
    timestamp_contract_address: Felt,
) -> u64 {
    let call_current_timestamp = FunctionCall {
        contract_address: timestamp_contract_address,
        entry_point_selector: get_selector_from_name("get_timestamp").unwrap(),
        calldata: vec![],
    };
    let call_result = devnet
        .json_rpc_client
        .call(call_current_timestamp, BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();
    assert_eq!(call_result.len(), 1);

    call_result[0].to_string().parse::<u64>().unwrap()
}

#[tokio::test]
async fn timestamp_syscall_set_in_past() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    // set time in past
    let past_time = 1;

    let block_timestamp = set_time(&devnet, past_time).await;
    assert_eq!(block_timestamp, past_time);
    devnet.create_block().await.unwrap();

    // check if timestamp is greater/equal
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_ge_with_buffer(current_timestamp, past_time);
}

#[tokio::test]
async fn timestamp_syscall_set_in_future() {
    let now = get_unix_timestamp_as_seconds();
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    // set time in future
    let future_time = now + 1000;
    let block_timestamp = set_time(&devnet, future_time).await;
    assert_eq!(block_timestamp, future_time);
    devnet.create_block().await.unwrap();

    // check if timestamp is greater/equal
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_ge_with_buffer(current_timestamp, future_time);
}

#[tokio::test]
async fn timestamp_syscall_increase_time() {
    let now = get_unix_timestamp_as_seconds();
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    // increase time
    let time_increment: u64 = 1000;

    increase_time(&devnet, time_increment).await;

    // check if timestamp is greater/equal
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_ge_with_buffer(current_timestamp, now + time_increment);

    sleep_until_new_timestamp().await;
    devnet.create_block().await.unwrap();

    // check if timestamp is greater
    let timestamp_after_new_block =
        get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_gt_with_buffer(timestamp_after_new_block, now + time_increment);
    assert_gt_with_buffer(timestamp_after_new_block, current_timestamp);
}

#[tokio::test]
async fn timestamp_syscall_contract_constructor() {
    let now = get_unix_timestamp_as_seconds();
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    sleep_until_new_timestamp().await;

    // check constructor set of timestamp
    let call_storage_timestamp = FunctionCall {
        contract_address: timestamp_contract_address,
        entry_point_selector: get_selector_from_name("get_storage_timestamp").unwrap(),
        calldata: vec![],
    };
    let storage_timestamp = devnet
        .json_rpc_client
        .call(call_storage_timestamp, BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap()[0]
        .to_string()
        .parse::<u64>()
        .unwrap();
    assert_gt_with_buffer(storage_timestamp, now);

    sleep_until_new_timestamp().await;
    devnet.create_block().await.unwrap();

    // check if current timestamp > storage timestamp and now
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_gt_with_buffer(current_timestamp, now);
    assert_gt_with_buffer(current_timestamp, storage_timestamp);
}

#[tokio::test]
async fn start_time_in_past_syscall() {
    let past_time = 1;
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        past_time.to_string().as_str(),
    ])
    .await
    .expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    devnet.create_block().await.unwrap();

    // check if timestamp is greater/equal
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_ge_with_buffer(current_timestamp, past_time);
}

#[tokio::test]
async fn start_time_in_future_syscall() {
    let now: u64 = get_unix_timestamp_as_seconds();
    let future_time = now + 1000;
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        future_time.to_string().as_str(),
    ])
    .await
    .expect("Could not start Devnet");
    let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

    devnet.create_block().await.unwrap();

    // check if timestamp is greater/equal
    let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
    assert_ge_with_buffer(current_timestamp, future_time);
}

async fn set_time_in_past(devnet: &BackgroundDevnet) {
    // set time and assert if >= past_time, check if inside buffer limit
    let past_time = 1;
    let block_timestamp = set_time(devnet, past_time).await;
    assert_eq!(block_timestamp, past_time);
    let set_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(set_time_block.timestamp, past_time);

    sleep_until_new_timestamp().await;

    // create block and check if block_timestamp > past_time, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(empty_block.timestamp, past_time);

    sleep_until_new_timestamp().await;

    // check if after create block timestamp > last block, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(latest_block.timestamp, empty_block.timestamp);
}

#[tokio::test]
async fn set_time_in_past_normal_mode() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    set_time_in_past(&devnet).await;
}

#[tokio::test]
async fn set_time_in_past_block_generation_on_demand() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .expect("Could not start Devnet");

    set_time_in_past(&devnet).await;
}

async fn set_time_in_future(devnet: &BackgroundDevnet) {
    // set time and assert if >= future_time, check if inside buffer limit
    let now = get_unix_timestamp_as_seconds();
    let future_time = now + 1000;
    let block_timestamp = set_time(devnet, future_time).await;
    assert_eq!(block_timestamp, future_time);
    let set_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(set_time_block.timestamp, future_time);

    sleep_until_new_timestamp().await;

    // create block and check if block_timestamp > future_time, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(empty_block.timestamp, future_time);

    sleep_until_new_timestamp().await;

    // check if after create block timestamp > last empty block, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(latest_block.timestamp, future_time);
}

#[tokio::test]
async fn set_time_in_future_normal_mode() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    set_time_in_future(&devnet).await;
}

#[tokio::test]
async fn set_time_in_future_block_generation_on_demand() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .expect("Could not start Devnet");

    set_time_in_future(&devnet).await;
}

#[tokio::test]
async fn set_time_with_pre_confirmed_txs() {
    let start_time = get_unix_timestamp_as_seconds();
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
        .await
        .unwrap();

    let mut sent_mint_txs = vec![];
    for _ in 0..2 {
        // dummy data
        let mint_tx = devnet.mint(Felt::ONE, 1_u128).await;
        sent_mint_txs.push(mint_tx);
    }
    sent_mint_txs.sort(); // sorting to allow equality assertion

    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    let mut pre_confirmed_txs = pre_confirmed_block.transactions.clone();
    pre_confirmed_txs.sort();
    assert_eq!(pre_confirmed_txs, sent_mint_txs);
    assert_eq_with_buffer(pre_confirmed_block.timestamp, start_time);

    let future_time = set_time(&devnet, start_time + 1000).await;

    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let mut latest_txs = latest_block.transactions.clone();
    latest_txs.sort();
    assert_eq!(latest_txs, sent_mint_txs);
    assert_ge_with_buffer(latest_block.timestamp, future_time);
}

#[tokio::test]
async fn set_time_empty_body() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let response_error = devnet.send_custom_rpc("devnet_setTime", json!({})).await.unwrap_err();
    assert_eq!(response_error.code, -32602);
}

#[tokio::test]
async fn set_time_wrong_body() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let rpc_error = devnet
        .send_custom_rpc(
            "devnet_setTime",
            json!({
                "test": 0
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(rpc_error.code, -32602);
}

#[tokio::test]
async fn test_increase_time() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let now = get_unix_timestamp_as_seconds();

    // increase time and assert if > now, check if inside buffer limit
    let first_increase_time: u64 = 10000;
    increase_time(&devnet, first_increase_time).await;
    let first_increase_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(first_increase_time_block.timestamp, now + first_increase_time);

    // second increase time, check if inside buffer limit
    let second_increase_time: u64 = 1000;
    increase_time(&devnet, second_increase_time).await;
    let second_increase_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(
        second_increase_time_block.timestamp,
        now + first_increase_time + second_increase_time,
    );

    sleep_until_new_timestamp().await;

    // create block and check again if block_timestamp > last block, check if
    // inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(empty_block.timestamp, second_increase_time_block.timestamp);

    sleep_until_new_timestamp().await;

    // check if after mint timestamp > last block, check if inside buffer limit
    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let last_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(last_block.timestamp, empty_block.timestamp);
}

#[tokio::test]
async fn increase_time_empty_body() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let rpc_error = devnet.send_custom_rpc("devnet_increaseTime", json!({})).await.unwrap_err();
    assert_eq!(rpc_error.code, -32602);
}

#[tokio::test]
async fn increase_time_wrong_body() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let rpc_error =
        devnet.send_custom_rpc("devnet_increaseTime", json!({ "test": 0 })).await.unwrap_err();
    assert_eq!(rpc_error.code, -32602);
}

#[tokio::test]
async fn wrong_start_time() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--start-time", "wrong"]).await;
    assert!(devnet.is_err());
}

#[tokio::test]
async fn start_time_in_past() {
    let past_time = 1;
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        past_time.to_string().as_str(),
    ])
    .await
    .expect("Could not start Devnet");

    // create block and check if block timestamp >= 1, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(empty_block.timestamp, past_time);
}

#[tokio::test]
async fn start_time_in_future() {
    let now = get_unix_timestamp_as_seconds();
    let future_time = now + 100;
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        future_time.to_string().as_str(),
    ])
    .await
    .expect("Could not start Devnet");

    // create block and check if block timestamp > now, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(empty_block.timestamp, future_time);
}

#[tokio::test]
async fn advance_time_combination_test_with_dump_and_load() {
    let now = get_unix_timestamp_as_seconds();
    let past_time = 1;
    let dump_file = UniqueAutoDeletableFile::new("time_combination");
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        past_time.to_string().as_str(),
        "--dump-path",
        dump_file.path.as_str(),
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    // increase time and assert if >= start-time argument + first_increase_time, check if inside
    // buffer limit
    let first_increase_time: u64 = 1000;
    increase_time(&devnet, first_increase_time).await;
    let first_increase_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(first_increase_time_block.timestamp, past_time + first_increase_time);

    // increase the time a second time and assert if >= past_time + first_increase_time +
    // second_increase_time, check if inside buffer limit
    let second_increase_time: u64 = 100;
    increase_time(&devnet, second_increase_time).await;
    let second_increase_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(
        second_increase_time_block.timestamp,
        past_time + first_increase_time + second_increase_time,
    );

    // set time to be now and check if the latest block timestamp >= now, check if
    // it's inside buffer limit
    let block_timestamp = set_time(&devnet, now).await;
    assert_eq!(block_timestamp, now);
    let set_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(set_time_block.timestamp, now);

    sleep_until_new_timestamp().await;

    // create a new empty block and check again if block timestamp > set_time_block, check if
    // inside buffer limit
    devnet.create_block().await.unwrap();
    let empty_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_gt_with_buffer(empty_block.timestamp, set_time_block.timestamp);

    // increase the time a third time and assert >= last empty block timestamp +
    // third_increase_time, check if inside buffer limit
    let third_increase_time: u64 = 10000;
    increase_time(&devnet, third_increase_time).await;
    let third_increase_time_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(
        third_increase_time_block.timestamp,
        empty_block.timestamp + third_increase_time,
    );

    sleep_until_new_timestamp().await;

    // check if the last block timestamp is > previous block, check if inside buffer limit
    devnet.create_block().await.unwrap();
    let last_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_ge_with_buffer(last_block.timestamp, third_increase_time_block.timestamp);

    send_ctrl_c_signal_and_wait(&devnet.process).await;

    // load from file and check block number and timestamp
    let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
        "--start-time",
        past_time.to_string().as_str(),
        "--dump-path",
        dump_file.path.as_str(),
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    let last_block_load = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
    assert_eq!(last_block.block_number, last_block_load.block_number);

    let timestamp_diff = last_block_load.timestamp.abs_diff(last_block.timestamp);
    assert!(timestamp_diff < BUFFER_TIME_SECONDS)
}

#[tokio::test]
async fn set_time_with_later_block_generation() {
    let now = get_unix_timestamp_as_seconds();
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--start-time", now.to_string().as_str()])
            .await
            .expect("Could not start Devnet");

    // set time in past without block generation
    let past_time = 1;
    let resp_body_set_time: serde_json::Value = devnet
        .send_custom_rpc("devnet_setTime", json!({ "time": past_time, "generate_block": false }))
        .await
        .unwrap();

    // time is set but the block was not generated
    assert_eq!(resp_body_set_time["block_timestamp"], past_time);
    assert!(resp_body_set_time["block_hash"].is_null());

    sleep_until_new_timestamp().await;

    // create block and assert
    devnet.create_block().await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    assert_eq!(latest_block.block_number, 1);
    assert_eq!(latest_block.timestamp, past_time);
}

#[tokio::test]

async fn correct_pending_block_timestamp() {
    let initial_time = get_unix_timestamp_as_seconds();
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--start-time", &initial_time.to_string()])
            .await
            .unwrap();

    let block = devnet.get_pre_confirmed_block_with_txs().await.unwrap();
    assert_eq!(block.timestamp, initial_time);
}

#[tokio::test]
async fn correct_pending_block_timestamp_after_setting() {
    let initial_time = get_unix_timestamp_as_seconds();
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--start-time", &initial_time.to_string()])
            .await
            .unwrap();

    let block = devnet.get_pre_confirmed_block_with_txs().await.unwrap();
    assert_eq!(block.timestamp, initial_time);

    sleep_until_new_timestamp().await;
    devnet.create_block().await.unwrap();

    let block = devnet.get_pre_confirmed_block_with_txs().await.unwrap();
    assert_gt_with_buffer(block.timestamp, initial_time);
}

#[tokio::test]
async fn tx_resource_estimation_fails_unless_time_incremented() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) = get_timestamp_asserter_contract_artifacts();

    let lock_interval = 86_400;
    let ctor_args = &[Felt::from(lock_interval)];
    let (_, contract_address) =
        declare_v3_deploy_v3(&account, contract_class, casm_hash, ctor_args).await.unwrap();

    let time_check_selector = get_selector_from_name("check_time").unwrap();
    let time_check_call =
        Call { to: contract_address, selector: time_check_selector, calldata: vec![] };

    // A failure is expected without time change.
    let error = account.execute_v3(vec![time_check_call.clone()]).estimate_fee().await.unwrap_err();
    match error {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(error_data),
        )) => {
            assert_eq!(error_data.transaction_index, 0);

            let root_error = extract_nested_error(&error_data.execution_error);
            assert_eq!(root_error.contract_address, account.address());
            assert_eq!(root_error.selector, get_selector_from_name("__execute__").unwrap());

            // Currently the root error is twice mentioned, so we extract twice
            let inner_error = extract_nested_error(&root_error.error);
            let inner_error = extract_nested_error(&inner_error.error);
            assert_eq!(inner_error.contract_address, contract_address);
            assert_eq!(inner_error.selector, time_check_selector);

            let message = extract_message_error(&inner_error.error);
            assert_contains(message, "Wait a bit more");
        }
        other => panic!("Invalid error: {other:?}"),
    }

    // Increasing the system timestamp should make the estimation succeed
    increase_time(&devnet, lock_interval).await;
    account.execute_v3(vec![time_check_call]).estimate_fee().await.unwrap();
}

#[tokio::test]
async fn tx_execution_fails_unless_time_incremented() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) = get_timestamp_asserter_contract_artifacts();

    let lock_interval = 86_400;
    let ctor_args = &[Felt::from(lock_interval)];
    let (_, contract_address) =
        declare_v3_deploy_v3(&account, contract_class, casm_hash, ctor_args).await.unwrap();

    let time_check_selector = get_selector_from_name("check_time").unwrap();
    let time_check_call =
        Call { to: contract_address, selector: time_check_selector, calldata: vec![] };

    // A failure is expected without time change.
    let reverted_tx = account
        .execute_v3(vec![time_check_call.clone()])
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(1e7 as u64)
        .send()
        .await
        .unwrap();

    match devnet.json_rpc_client.get_transaction_status(reverted_tx.transaction_hash).await {
        Ok(TransactionStatus::AcceptedOnL2(tx_details)) => {
            assert_eq!(tx_details.status(), TransactionExecutionStatus::Reverted);
            assert_contains(tx_details.revert_reason().unwrap(), "Wait a bit more");
        }
        other => panic!("Unexpected tx: {other:?}"),
    }

    // Increasing the system timestamp should make the tx succeed (and the implicit fee estimation)
    increase_time(&devnet, lock_interval).await;
    account.execute_v3(vec![time_check_call]).send().await.unwrap();
}
