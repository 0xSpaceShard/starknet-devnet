use std::path::Path;
use std::time;

use serde_json::json;
use starknet_rs_providers::Provider;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::reqwest_client::PostReqwestSender;
use crate::common::utils::{send_ctrl_c_signal_and_wait, FeeUnit, UniqueAutoDeletableFile};

static DUMMY_ADDRESS: u128 = 1;
static DUMMY_AMOUNT: u128 = 1;

use std::sync::Arc;

use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{DeclareTransaction, Felt, InvokeTransaction, Transaction};

use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

async fn dump_load_dump_load(mode: &str) {
    let dump_file =
        UniqueAutoDeletableFile::new(("dump_load_dump_load_on_".to_owned() + mode).as_str());

    for _ in 0..2 {
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            mode,
        ])
        .await
        .expect("Could not start Devnet");

        devnet_dump.create_block().await.unwrap();
        devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        send_ctrl_c_signal_and_wait(&devnet_dump.process).await;
    }

    let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        mode,
    ])
    .await
    .expect("Could not start Devnet");

    let last_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
    assert_eq!(last_block.block_number, 4);
}

#[tokio::test]
async fn dump_load_dump_load_without_path() {
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&["--dump-on", "request"])
        .await
        .expect("Could not start Devnet");

    for _ in 0..2 {
        devnet_dump.create_block().await.unwrap();
        devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    }
    let dump_rpc = devnet_dump.send_custom_rpc("devnet_dump", json!({})).await.unwrap().to_string();
    let dump_file = UniqueAutoDeletableFile::new("dump_load_dump_load_on_request_nofile");
    std::fs::write(&dump_file.path, dump_rpc).expect("Failed to write dump file");

    let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "request",
    ])
    .await
    .expect("Could not start Devnet");

    let last_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
    assert_eq!(last_block.block_number, 4);

    let loaded_balance =
        devnet_load.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Wei).await.unwrap();
    assert_eq!(loaded_balance, Felt::from(DUMMY_AMOUNT * 2));
}

#[tokio::test]
async fn dump_load_dump_load_on_exit() {
    dump_load_dump_load("exit").await;
}

#[tokio::test]
async fn dump_load_dump_load_on_transaction() {
    dump_load_dump_load("block").await;
}

#[tokio::test]
async fn dump_wrong_cli_parameters_path() {
    let devnet_dump =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", "///", "--dump-on", "block"])
            .await;

    assert!(devnet_dump.is_err());
}

#[tokio::test]
async fn dump_and_load_blocks_generation_on_demand() {
    let modes = vec!["exit", "block"];

    for mode in modes {
        let dump_file =
            UniqueAutoDeletableFile::new(("dump_load_dump_load_on_".to_owned() + mode).as_str());

        let total_iterations = 2;
        for _ in 0..total_iterations {
            let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
                "--dump-path",
                &dump_file.path,
                "--dump-on",
                mode,
                "--block-generation-on",
                "demand",
            ])
            .await
            .expect("Could not start Devnet");

            devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            devnet_dump.create_block().await.unwrap();

            send_ctrl_c_signal_and_wait(&devnet_dump.process).await;
        }

        let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            mode,
            "--block-generation-on",
            "demand",
        ])
        .await
        .expect("Could not start Devnet");

        let last_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(last_block.block_number, total_iterations);
        assert_eq!(last_block.transactions.len(), total_iterations as usize);
    }
}

#[tokio::test]
async fn mint_dump_on_transaction_and_load() {
    // dump after transaction
    let dump_file = UniqueAutoDeletableFile::new("dump_on_transaction");
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "block",
    ])
    .await
    .expect("Could not start Devnet");
    let mint_tx_hash_1 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let mint_tx_hash_2 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

    // load transaction from file and check transaction hash
    let devnet_load =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
            .await
            .expect("Could not start Devnet");
    let loaded_tx_1 =
        devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_1).await.unwrap();
    if let Transaction::Invoke(InvokeTransaction::V3(tx)) = loaded_tx_1 {
        assert_eq!(tx.transaction_hash, mint_tx_hash_1);
    } else {
        panic!("Could not unpack the transaction from {loaded_tx_1:?}");
    }

    let loaded_tx_2 =
        devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_2).await.unwrap();
    if let Transaction::Invoke(InvokeTransaction::V3(tx)) = loaded_tx_2 {
        assert_eq!(tx.transaction_hash, mint_tx_hash_2);
    } else {
        panic!("Could not unpack the transaction from {loaded_tx_2:?}");
    }
}

#[tokio::test]
async fn mint_dump_on_exit_and_load() {
    // dump on exit
    let dump_file = UniqueAutoDeletableFile::new("dump_on_exit");
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        dump_file.path.as_str(),
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");
    let devnet_dump_pid = devnet_dump.process.id();
    let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

    send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

    // load transaction from file and check transaction hash
    let devnet_load =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
            .await
            .expect("Could not start Devnet");
    let devnet_load_pid = devnet_load.process.id();
    assert_ne!(devnet_dump_pid, devnet_load_pid); // if PID's are different SIGINT signal worked
    let loaded_transaction =
        devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
    if let Transaction::Invoke(InvokeTransaction::V3(tx)) = loaded_transaction {
        assert_eq!(tx.transaction_hash, mint_tx_hash);
    } else {
        panic!("Could not unpack the transaction from {loaded_transaction:?}");
    }
}

#[tokio::test]
async fn declare_deploy() {
    let dump_file = UniqueAutoDeletableFile::new("dump_declare_deploy");
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "block",
    ])
    .await
    .expect("Could not start Devnet");

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (cairo_1_contract, casm_class_hash) =
        get_events_contract_in_sierra_and_compiled_class_hash();

    // declare the contract
    let declaration_result = predeployed_account
        .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
        .max_fee(Felt::from(1e18 as u128))
        .send()
        .await
        .unwrap();

    let predeployed_account = Arc::new(predeployed_account);

    // deploy the contract
    let contract_factory =
        ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
    let deploy_result = contract_factory
        .deploy_v1(vec![], Felt::ZERO, false)
        .max_fee(Felt::from(1e18 as u128))
        .send()
        .await
        .unwrap();

    // load transaction from file and check transactions hashes
    let devnet_load =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
            .await
            .expect("Could not start Devnet");

    // check declare transaction
    let loaded_declare_v2 = devnet_load
        .json_rpc_client
        .get_transaction_by_hash(declaration_result.transaction_hash)
        .await
        .unwrap();
    if let Transaction::Declare(DeclareTransaction::V2(tx)) = loaded_declare_v2 {
        assert_eq!(tx.transaction_hash, declaration_result.transaction_hash);
    } else {
        panic!("Could not unpack the transaction from {loaded_declare_v2:?}");
    }

    // check deploy transaction
    let loaded_deploy_v2 = devnet_load
        .json_rpc_client
        .get_transaction_by_hash(deploy_result.transaction_hash)
        .await
        .unwrap();
    if let Transaction::Invoke(InvokeTransaction::V1(tx)) = loaded_deploy_v2 {
        assert_eq!(tx.transaction_hash, deploy_result.transaction_hash);
    } else {
        panic!("Could not unpack the transaction from {loaded_deploy_v2:?}");
    }
}

#[tokio::test]
async fn dump_without_transaction() {
    // dump on exit
    let dump_file = UniqueAutoDeletableFile::new("dump_without_transaction");
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

    // file should not be created if there are no transactions
    if Path::new(&dump_file.path).exists() {
        panic!(
            "Could find the dump file but there were no transactions to dump {}",
            &dump_file.path
        );
    }
}

#[tokio::test]
async fn dump_endpoint_fail_with_no_mode_set() {
    let devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let rpc_error = devnet_dump.send_custom_rpc("devnet_dump", json!({})).await.unwrap_err();
    assert!(rpc_error.message.contains("Please provide --dump-on mode"));
}

#[tokio::test]
async fn dump_endpoint_fail_with_wrong_request() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let err = devnet.send_custom_rpc("devnet_dump", json!({ "test": "" })).await.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn dump_endpoint_fail_with_wrong_file_name() {
    let dump_file = UniqueAutoDeletableFile::new("dump_wrong_file_name");
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    let err = devnet.send_custom_rpc("devnet_dump", json!({ "path": "///" })).await.unwrap_err();
    assert!(err.message.contains("I/O error"));
}

#[tokio::test]
async fn load_endpoint_fail_with_wrong_request() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let err = devnet.send_custom_rpc("devnet_load", json!({ "test": "" })).await.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn load_endpoint_fail_with_wrong_path() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let err = devnet
        .send_custom_rpc("devnet_load", json!({ "path": "load_file_name" }))
        .await
        .unwrap_err();
    assert!(err.message.contains("file does not exist"));
}

#[tokio::test]
async fn dump_load_endpoints_transaction_and_state_after_load_is_valid() {
    // check if the dump with the default path "dump_endpoint" works as expected when json body
    // is empty, later check if the dump with the custom path "dump_endpoint_custom_path"
    // works
    let dump_file = UniqueAutoDeletableFile::new("dump_endpoint");
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    devnet_dump.send_custom_rpc("devnet_dump", json!({})).await.unwrap();
    assert!(Path::new(&dump_file.path).exists());

    let dump_file_custom = UniqueAutoDeletableFile::new("dump_endpoint_custom_path");
    devnet_dump
        .send_custom_rpc("devnet_dump", json!({ "path": dump_file_custom.path }))
        .await
        .unwrap();
    assert!(Path::new(&dump_file_custom.path).exists());

    // load and re-execute from "dump_endpoint" file and check if transaction and state of the
    // blockchain is valid
    let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    devnet_load.send_custom_rpc("devnet_load", json!({ "path": dump_file.path })).await.unwrap();

    let balance_result =
        devnet_load.get_balance_latest(&Felt::from(DUMMY_ADDRESS), FeeUnit::Wei).await.unwrap();
    assert_eq!(balance_result, Felt::from(DUMMY_AMOUNT));

    let loaded_transaction =
        devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
    if let Transaction::Invoke(InvokeTransaction::V3(tx)) = loaded_transaction {
        assert_eq!(tx.transaction_hash, mint_tx_hash);
    } else {
        panic!("Could not unpack the transaction from {loaded_transaction:?}");
    }
}

#[tokio::test]
async fn mint_and_dump_and_load_on_same_devnet() {
    let dump_file = UniqueAutoDeletableFile::new("dump");
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-on",
        "exit",
        "--dump-path",
        &dump_file.path,
    ])
    .await
    .unwrap();

    let unit = FeeUnit::Wei;

    devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
    let balance_before_dump = devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
    assert_eq!(balance_before_dump, Felt::from(DUMMY_AMOUNT));

    devnet.send_custom_rpc("devnet_dump", json!({ "path": dump_file.path })).await.unwrap();

    devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
    let balance_after_dump = devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
    assert_eq!(balance_after_dump, balance_before_dump + Felt::from(DUMMY_AMOUNT));

    devnet.send_custom_rpc("devnet_load", json!({ "path": dump_file.path })).await.unwrap();

    let balance_after_load = devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
    assert_eq!(balance_after_load, balance_before_dump);

    devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
    let balance_after_mint_on_loaded =
        devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
    assert_eq!(balance_after_mint_on_loaded, balance_after_load + Felt::from(DUMMY_AMOUNT));
}

#[tokio::test]
async fn set_time_with_later_block_generation_dump_and_load() {
    let dump_file = UniqueAutoDeletableFile::new("dump_set_time");
    let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "exit",
    ])
    .await
    .expect("Could not start Devnet");

    // set time in past without block generation
    let past_time = 1;
    devnet_dump
        .send_custom_rpc("devnet_setTime", json!({ "time": past_time, "generate_block": false }))
        .await
        .unwrap();

    // wait 1 second
    tokio::time::sleep(time::Duration::from_secs(1)).await;

    devnet_dump.create_block().await.unwrap();
    devnet_dump.get_latest_block_with_tx_hashes().await.unwrap();

    // dump and load
    send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

    // load and assert
    let devnet_load =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
            .await
            .expect("Could not start Devnet");

    let latest_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();

    assert_eq!(latest_block.block_number, 1);
    assert_eq!(latest_block.timestamp, past_time);
}

/// Ever since the introduction of non-rpc to rpc mapper, it is worth testing if non-rpc
/// requests do what we want. Especially since the vast majority of our e2e tests
/// rely on the JSON-RPC API.
#[tokio::test]
async fn test_dumping_of_non_rpc_requests() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-on",
        "request",
        "--state-archive-capacity",
        "full",
    ])
    .await
    .unwrap();

    // mint, create block, abort block, create block
    let address = "0x1";
    let mint_amount = 100;
    let _: serde_json::Value = devnet
        .reqwest_client()
        .post_json_async("/mint", json!({ "address": address, "amount": mint_amount }))
        .await
        .unwrap();

    let first_created_block: serde_json::Value =
        devnet.reqwest_client().post_json_async("/create_block", json!({})).await.unwrap();

    let second_created_block: serde_json::Value =
        devnet.reqwest_client().post_json_async("/create_block", json!({})).await.unwrap();

    let _: serde_json::Value = devnet
        .reqwest_client()
        .post_json_async(
            "/abort_blocks",
            json!({ "starting_block_id": { "block_hash": second_created_block["block_hash"] } }),
        )
        .await
        .unwrap();

    // dump and spawn a new devnet by loading
    let dump_file = UniqueAutoDeletableFile::new("non-rpc-dump");
    devnet.send_custom_rpc("devnet_dump", json!({ "path": dump_file.path })).await.unwrap();

    let loaded_devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--state-archive-capacity",
        "full",
    ])
    .await
    .unwrap();

    let loaded_balance = loaded_devnet
        .get_balance_latest(&Felt::from_hex_unchecked(address), FeeUnit::Wei)
        .await
        .unwrap();
    assert_eq!(loaded_balance, Felt::from(mint_amount));

    let loaded_latest_block = loaded_devnet.get_latest_block_with_tx_hashes().await.unwrap();
    assert_eq!(
        loaded_latest_block.block_hash,
        Felt::from_hex_unchecked(first_created_block["block_hash"].as_str().unwrap())
    );
}

#[tokio::test]
async fn test_dumping_after_restart() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-on", "request"]).await.unwrap();

    // mint, restart, assert dump empty
    let address = Felt::ONE;
    let mint_amount = 100;
    devnet.mint(address, mint_amount).await;

    devnet.restart().await;

    let dump_resp = devnet.send_custom_rpc("devnet_dump", serde_json::Value::Null).await.unwrap();
    assert_eq!(dump_resp, json!([]));
}
