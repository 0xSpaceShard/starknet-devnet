//! This module tests the messaging feature from the starknet-server perspective.
//!
//! The message contract `cairo_0_l1l2_contract` associates a balance to a user (ContractAddress)
//! and contains the following entrypoints:
//! * increase_balance -> increases the balance of a user (contract address) for the given amount.
//! * get_balance -> gets the balance of a user.
//! * withdraw -> withdraw from a user the given amount, sending the amount to a l2->l1 message.
//! * deposit -> deposit the given amount from a l1->l2 message (l1 handler function).

use std::str::FromStr;
use std::sync::Arc;

use ethers::prelude::*;
use serde_json::{json, Value};
use server::test_utils::assert_contains;
use starknet_rs_accounts::{
    Account, AccountError, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, Felt, FunctionCall, InvokeTransactionResult,
    TransactionExecutionStatus, TransactionReceipt, TransactionReceiptWithBlockInfo,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address, UdcUniqueness};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_rs_signers::LocalWallet;

use crate::common::background_anvil::BackgroundAnvil;
use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    CHAIN_ID, L1_HANDLER_SELECTOR, MESSAGING_L1_CONTRACT_ADDRESS, MESSAGING_L2_CONTRACT_ADDRESS,
    MESSAGING_WHITELISTED_L1_CONTRACT,
};
use crate::common::errors::RpcError;
use crate::common::utils::{
    assert_tx_successful, felt_to_u256, get_messaging_contract_in_sierra_and_compiled_class_hash,
    get_messaging_lib_in_sierra_and_compiled_class_hash, send_ctrl_c_signal_and_wait, to_hex_felt,
    UniqueAutoDeletableFile,
};

const DUMMY_L1_ADDRESS: &str = "0xc662c410c0ecf747543f5ba90660f6abebd9c8c4";
const MESSAGE_WITHDRAW_OPCODE: &str = "0x0";

const MAX_FEE: u128 = 1e18 as u128;

/// Differs from MESSAGING_WHITELISTED_L1_CONTRACT: that address is hardcoded in the cairo0 l1l2
/// contract relying on it This address is provided as an argument to the cairo1 l1l2
/// contract
const MESSAGING_L1_ADDRESS: &str = "0x5fbdb2315678afecb367f032d93f642f64180aa3";

/// Withdraws the given amount from a user and send this amount in a l2->l1 message.
async fn withdraw<A: ConnectedAccount + Send + Sync + 'static>(
    account: A,
    contract_address: Felt,
    user: Felt,
    amount: Felt,
    l1_address: Felt,
) {
    let invoke_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("withdraw").unwrap(),
        calldata: vec![user, amount, l1_address],
    }];

    account.execute_v1(invoke_calls).max_fee(Felt::from(MAX_FEE)).send().await.unwrap();
}

/// Increases the balance for the given user.
async fn increase_balance<A: ConnectedAccount + Send + Sync + 'static>(
    account: A,
    contract_address: Felt,
    user: Felt,
    amount: Felt,
) {
    let invoke_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![user, amount],
    }];

    account.execute_v1(invoke_calls).max_fee(Felt::from(MAX_FEE)).send().await.unwrap();
}

/// Gets the balance for the given user.
async fn get_balance(devnet: &BackgroundDevnet, contract_address: Felt, user: Felt) -> Vec<Felt> {
    let call = FunctionCall {
        contract_address,
        entry_point_selector: get_selector_from_name("get_balance").unwrap(),
        calldata: vec![user],
    };

    devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Pending)).await.unwrap()
}

/// Withdraws the given amount from a user and send this amount in a l2->l1 message
/// using a library syscall instead of the contract with storage directly.
async fn withdraw_from_lib<A: ConnectedAccount + Send + Sync + 'static>(
    account: A,
    contract_address: Felt,
    user: Felt,
    amount: Felt,
    l1_address: Felt,
    lib_class_hash: Felt,
) -> Result<InvokeTransactionResult, AccountError<<A as Account>::SignError>> {
    let invoke_calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("withdraw_from_lib").unwrap(),
        calldata: vec![user, amount, l1_address, lib_class_hash],
    }];

    account.execute_v1(invoke_calls).max_fee(Felt::from(MAX_FEE)).send().await
}

/// Returns the deployment address
async fn deploy_l2_msg_contract(
    account: Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
) -> Result<Felt, anyhow::Error> {
    // Declare l1l2 contract with storage (meant to be deployed).
    let (sierra_class, casm_class_hash) =
        get_messaging_contract_in_sierra_and_compiled_class_hash();

    let sierra_class_hash = sierra_class.class_hash();
    let declaration = account.declare_v2(Arc::new(sierra_class), casm_class_hash);
    declaration.max_fee(Felt::from(MAX_FEE)).send().await?;

    // deploy instance of class
    let contract_factory = ContractFactory::new(sierra_class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![];
    let contract_address = get_udc_deployed_address(
        salt,
        sierra_class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v1(constructor_calldata, salt, false)
        .nonce(Felt::ONE)
        .max_fee(Felt::from(MAX_FEE))
        .send()
        .await
        .unwrap();

    Ok(contract_address)
}

/// Sets up a `BackgroundDevnet` with the message l1-l2 contract deployed.
/// Returns (devnet instance, account used for deployment, l1-l2 contract address).
async fn setup_devnet(
    devnet_args: &[&str],
) -> (BackgroundDevnet, Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>, Felt) {
    let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    let contract_address = deploy_l2_msg_contract(account.clone()).await.unwrap();

    (devnet, account, contract_address)
}

fn assert_traces(traces: &Value) {
    assert_eq!(traces["type"], "L1_HANDLER");
    assert_eq!(traces["function_invocation"]["contract_address"], MESSAGING_L2_CONTRACT_ADDRESS);
    assert_eq!(traces["function_invocation"]["entry_point_selector"], L1_HANDLER_SELECTOR);
    assert_eq!(traces["function_invocation"]["calldata"][0], MESSAGING_L1_CONTRACT_ADDRESS);
    assert!(traces["state_diff"].is_object());
}

#[tokio::test]
async fn can_send_message_to_l1() {
    let (devnet, account, l1l2_contract_address) =
        setup_devnet(&["--account-class", "cairo1"]).await;
    let user = Felt::ONE;

    // Set balance to 1 for user.
    let user_balance = Felt::ONE;
    increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

    // We don't actually send a message to the L1 in this test.
    let l1_address = Felt::from_hex_unchecked(DUMMY_L1_ADDRESS);

    // Withdraw the 1 amount in a l2->l1 message.
    withdraw(Arc::clone(&account), l1l2_contract_address, user, user_balance, l1_address).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [Felt::ZERO]);

    // Flush messages to check the presence of the withdraw
    let resp_body: serde_json::Value =
        devnet.send_custom_rpc("devnet_postmanFlush", json!({ "dry_run": true })).await.unwrap();

    assert_eq!(
        resp_body,
        json!({
            "messages_to_l1": [
                {
                    "from_address": to_hex_felt(&l1l2_contract_address),
                    "to_address": DUMMY_L1_ADDRESS,
                    "payload": [MESSAGE_WITHDRAW_OPCODE, &to_hex_felt(&user), &to_hex_felt(&user_balance)]
                }
            ],
            "messages_to_l2": [],
            "generated_l2_transactions": [],
            "l1_provider": "dry run"
        })
    );
}

#[tokio::test]
async fn can_send_message_to_l1_from_library_syscall() {
    let (devnet, account, l1l2_contract_address) =
        setup_devnet(&["--account-class", "cairo1"]).await;

    // Declare l1l2 lib with only one function to send messages.
    // Its class hash can then be ignored, it's hardcoded in the contract.
    let (sierra_class, casm_class_hash) = get_messaging_lib_in_sierra_and_compiled_class_hash();
    let lib_sierra_class_hash = sierra_class.class_hash();

    account
        .declare_v2(Arc::new(sierra_class), casm_class_hash)
        .max_fee(Felt::from(MAX_FEE))
        .send()
        .await
        .unwrap();

    let user = Felt::ONE;

    // Set balance to 1 for user.
    let user_balance = Felt::ONE;
    increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

    // We don't actually send a message to the L1 in this test.
    let l1_address = Felt::from_hex_unchecked(DUMMY_L1_ADDRESS);

    // Withdraw the 1 amount in an l2->l1 message.
    withdraw_from_lib(
        Arc::clone(&account),
        l1l2_contract_address,
        user,
        user_balance,
        l1_address,
        lib_sierra_class_hash,
    )
    .await
    .unwrap();
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [Felt::ZERO]);

    // Flush messages to check the presence of the withdraw
    let resp_body: serde_json::Value =
        devnet.send_custom_rpc("devnet_postmanFlush", json!({ "dry_run": true })).await.unwrap();

    assert_eq!(
        resp_body,
        json!({
            "messages_to_l1": [
                {
                    "from_address": to_hex_felt(&l1l2_contract_address),
                    "to_address": DUMMY_L1_ADDRESS,
                    "payload": [MESSAGE_WITHDRAW_OPCODE, &to_hex_felt(&user), &to_hex_felt(&user_balance)]
                }
            ],
            "messages_to_l2": [],
            "generated_l2_transactions": [],
            "l1_provider": "dry run"
        })
    );
}

#[tokio::test]
async fn mock_message_to_l2_creates_a_tx_with_desired_effect() {
    let (devnet, account, l1l2_contract_address) =
        setup_devnet(&["--account-class", "cairo1"]).await;
    let user = Felt::ONE;

    // Set balance to 1 for user.
    let user_balance = Felt::ONE;
    increase_balance(account, l1l2_contract_address, user, user_balance).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

    // Use postman to send a message to l2 without l1 - the message increments user balance
    let increment_amount = Felt::from_hex_unchecked("0xff");

    let body: serde_json::Value = devnet.send_custom_rpc("devnet_postmanSendMessageToL2", json!({
            "l1_contract_address": MESSAGING_L1_ADDRESS,
            "l2_contract_address": format!("0x{:64x}", l1l2_contract_address),
            "entry_point_selector": format!("0x{:64x}", get_selector_from_name("deposit").unwrap()),
            "payload": [to_hex_felt(&user), to_hex_felt(&increment_amount)],
            "paid_fee_on_l1": "0x1234",
            "nonce": "0x1"
        }))
        .await.unwrap();
    let tx_hash_hex = body.get("transaction_hash").unwrap().as_str().unwrap();
    let tx_hash = Felt::from_hex_unchecked(tx_hash_hex);
    assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;

    // assert state changed
    assert_eq!(
        get_balance(&devnet, l1l2_contract_address, user).await,
        [user_balance + increment_amount]
    );

    // assert tx and receipt retrievable and correct
    let expected_calldata =
        vec![Felt::from_hex_unchecked(MESSAGING_L1_ADDRESS), user, increment_amount];
    match devnet.json_rpc_client.get_transaction_by_hash(tx_hash).await {
        Ok(starknet_rs_core::types::Transaction::L1Handler(tx)) => {
            assert_eq!(tx.transaction_hash, tx_hash);
            assert_eq!(tx.calldata, expected_calldata);
        }
        other => panic!("Error in fetching tx: {other:?}"),
    }
    match devnet.json_rpc_client.get_transaction_receipt(tx_hash).await {
        Ok(TransactionReceiptWithBlockInfo {
            receipt: TransactionReceipt::L1Handler(receipt),
            ..
        }) => {
            assert_eq!(receipt.transaction_hash, tx_hash);
            assert_eq!(receipt.execution_result.status(), TransactionExecutionStatus::Succeeded);
        }
        other => panic!("Error in fetching receipt: {other:?}"),
    }
}

#[tokio::test]
async fn can_deploy_l1_messaging_contract() {
    let anvil = BackgroundAnvil::spawn().await.unwrap();

    let (devnet, _, _) = setup_devnet(&["--account-class", "cairo1"]).await;

    let body = devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .expect("deploy l1 messaging contract failed");

    assert_eq!(
        body.get("messaging_contract_address").unwrap().as_str().unwrap(),
        MESSAGING_L1_ADDRESS
    );
}

#[tokio::test]
async fn can_consume_from_l2() {
    let (devnet, account, l1l2_contract_address) =
        setup_devnet(&["--account-class", "cairo1"]).await;
    let user = Felt::ONE;

    // Set balance to 1 for user.
    let user_balance = Felt::ONE;
    increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

    // We don't need valid l1 address as we don't use L1 node.
    let l1_address = Felt::from_hex_unchecked(DUMMY_L1_ADDRESS);

    // Withdraw the 1 amount in a l2->l1 message.
    withdraw(Arc::clone(&account), l1l2_contract_address, user, user_balance, l1_address).await;
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [Felt::ZERO]);

    let body = devnet
        .send_custom_rpc(
            "devnet_postmanConsumeMessageFromL2",
            json!({
                "from_address": format!("0x{:64x}", l1l2_contract_address),
                "to_address": DUMMY_L1_ADDRESS,
                "payload": ["0x0","0x1","0x1"],
            }),
        )
        .await
        .expect("consume message from l2 failed");
    assert_eq!(
        body.get("message_hash").unwrap().as_str().unwrap(),
        "0xac5356289f98e5012f8efcabb9f7ad3adaa43f81ac47b6311d2c23ca01590bd0"
    );
}

#[tokio::test]
async fn can_interact_with_l1() {
    let dump_file = UniqueAutoDeletableFile::new("can_interact_with_l1");
    let anvil = BackgroundAnvil::spawn().await.unwrap();
    let (devnet, sn_account, sn_l1l2_contract) = setup_devnet(&[
        "--account-class",
        "cairo1",
        "--dump-path",
        &dump_file.path,
        "--dump-on",
        "exit",
    ])
    .await;

    // Load l1 messaging contract.
    let body: serde_json::Value = devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .expect("deploy l1 messaging contract failed");

    assert_eq!(
        body.get("messaging_contract_address").unwrap().as_str().unwrap(),
        MESSAGING_L1_ADDRESS
    );

    // Deploy the L1L2 testing contract on L1 (on L2 it's already pre-deployed).
    let l1_messaging_address = H160::from_str(MESSAGING_L1_ADDRESS).unwrap();
    let eth_l1l2_address = anvil.deploy_l1l2_contract(l1_messaging_address).await.unwrap();
    let eth_l1l2_address_hex = format!("{eth_l1l2_address:#x}");

    let eth_l1l2_address_felt = Felt::from_hex_unchecked(&eth_l1l2_address_hex);
    let user_sn = Felt::ONE;
    let user_eth: U256 = 1.into();

    // Set balance to 1 for the user 1 on L2.
    let user_balance = Felt::ONE;
    increase_balance(sn_account.clone(), sn_l1l2_contract, user_sn, user_balance).await;
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [user_balance]);

    // Withdraw the amount 1 from user 1 balance on L2 to send it on L1 with a l2->l1 message.
    withdraw(sn_account, sn_l1l2_contract, user_sn, user_balance, eth_l1l2_address_felt).await;
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    // Flush to send the messages.
    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.expect("flush failed");

    // Check that the balance is 0 on L1 before consuming the message.
    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, 0.into());

    let sn_l1l2_contract_u256 = felt_to_u256(sn_l1l2_contract);

    // Consume the message to increase the balance.
    anvil.withdraw_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into()).await.unwrap();

    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, 1.into());

    // Send back the amount 1 to the user 1 on L2.
    anvil.deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into()).await.unwrap();

    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();

    // Balances on both layers is 0 at this point.
    assert_eq!(user_balance_eth, 0.into());
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    // Flush messages to have MessageToL2 executed.
    let flush_resp = devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();
    let generated_l2_txs = flush_resp["generated_l2_transactions"].as_array().unwrap();
    assert_eq!(generated_l2_txs.len(), 1); // expect this to be the only tx
    let generated_l2_tx = &generated_l2_txs[0];

    // Ensure the balance is back to 1 on L2.
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ONE]);

    // Assert traces of L1Handler transaction with custom rpc call,
    // json_rpc_client.trace_transaction() is not supported
    let l1_handler_tx_trace = &devnet
        .send_custom_rpc(
            "starknet_traceTransaction",
            json!({ "transaction_hash": generated_l2_tx }),
        )
        .await
        .unwrap();
    assert_traces(l1_handler_tx_trace);

    send_ctrl_c_signal_and_wait(&devnet.process).await;

    // Assert traces of L1Handler with loaded devnets
    let load_devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--account-class",
        "cairo1",
    ])
    .await
    .unwrap();

    let l1_handler_tx_trace_load = &load_devnet
        .send_custom_rpc(
            "starknet_traceTransaction",
            json!({ "transaction_hash": generated_l2_tx }),
        )
        .await
        .unwrap();
    assert_traces(l1_handler_tx_trace_load);
}

#[tokio::test]
async fn assert_l1_handler_tx_can_be_dumped_and_loaded() {
    let dump_file = UniqueAutoDeletableFile::new("dump-with-l1-handler");
    let (dumping_devnet, account, l1l2_contract_address) = setup_devnet(&[
        "--account-class",
        "cairo1",
        "--dump-on",
        "exit",
        "--dump-path",
        &dump_file.path,
    ])
    .await;

    // Set balance for user
    let user = Felt::ONE;
    let user_balance = Felt::ONE;
    increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
    assert_eq!(get_balance(&dumping_devnet, l1l2_contract_address, user).await, [user_balance]);

    // Use postman to send a message to l2 without l1 - the message increments user balance
    let increment_amount = Felt::from_hex_unchecked("0xff");

    dumping_devnet.send_custom_rpc("devnet_postmanSendMessageToL2", json!({
            "l1_contract_address": MESSAGING_WHITELISTED_L1_CONTRACT,
            "l2_contract_address": format!("0x{:64x}", l1l2_contract_address),
            "entry_point_selector": format!("0x{:64x}", get_selector_from_name("deposit").unwrap()),
            "payload": [to_hex_felt(&user), to_hex_felt(&increment_amount)],
            "paid_fee_on_l1": "0x1234",
            "nonce": "0x1"
        })).await.unwrap();

    assert_eq!(
        get_balance(&dumping_devnet, l1l2_contract_address, user).await,
        [user_balance + increment_amount]
    );

    send_ctrl_c_signal_and_wait(&dumping_devnet.process).await;

    let loading_devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        &dump_file.path,
        "--account-class",
        "cairo1",
    ])
    .await
    .unwrap();

    assert_eq!(
        get_balance(&loading_devnet, l1l2_contract_address, user).await,
        [user_balance + increment_amount]
    );
}

#[tokio::test]
async fn test_correct_message_order() {
    let anvil = BackgroundAnvil::spawn_with_additional_args(&["--block-time", "1"]).await.unwrap();
    let (devnet, sn_account, sn_l1l2_contract) = setup_devnet(&[]).await;

    // Load l1 messaging contract.
    let load_resp: serde_json::Value = devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .expect("deploy l1 messaging contract failed");

    assert_eq!(
        load_resp.get("messaging_contract_address").unwrap().as_str().unwrap(),
        MESSAGING_L1_ADDRESS
    );

    // Deploy the L1L2 testing contract on L1 (on L2 it's already pre-deployed).
    let l1_messaging_address = H160::from_str(MESSAGING_L1_ADDRESS).unwrap();
    let eth_l1l2_address = anvil.deploy_l1l2_contract(l1_messaging_address).await.unwrap();
    let eth_l1l2_address_hex = format!("{eth_l1l2_address:#x}");

    let eth_l1l2_address_felt = Felt::from_hex_unchecked(&eth_l1l2_address_hex);
    let user_sn = Felt::ONE;
    let user_eth: U256 = 1.into();

    // Set balance for the user on L2.
    let init_balance = 5_u64;
    increase_balance(sn_account.clone(), sn_l1l2_contract, user_sn, init_balance.into()).await;

    // Withdraw the set amount from user 1 balance on L2 to send it on L1 with a l2->l1 message.
    withdraw(sn_account, sn_l1l2_contract, user_sn, init_balance.into(), eth_l1l2_address_felt)
        .await;

    // Flush to send the messages.
    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.expect("flush failed");

    let sn_l1l2_contract_u256 = felt_to_u256(sn_l1l2_contract);

    // Consume the message to increase the L1 balance.
    anvil
        .withdraw_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, init_balance.into())
        .await
        .unwrap();

    // Send back an amount of 1 to the user on L2. Do it n times to have n transactions,
    // for the purpose of message order testing (n = init_balance)
    for _ in 0..init_balance {
        anvil
            .deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into())
            .await
            .unwrap();
    }

    // Flush messages to have MessageToL2 executed.
    let flush_resp = devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();
    let generated_l2_txs = flush_resp["messages_to_l2"].as_array().unwrap();

    let flushed_message_nonces: Vec<_> = generated_l2_txs
        .iter()
        .map(|msg| msg["nonce"].as_str().unwrap())
        .map(|nonce| u64::from_str_radix(nonce.strip_prefix("0x").unwrap(), 16).unwrap())
        .collect();

    let expected_nonces: Vec<_> = (0..init_balance).collect();
    assert_eq!(flushed_message_nonces, expected_nonces);
}

#[tokio::test]
/// Here we test if the devnet_postmanLoad method call is indeed dumped and then used in a load.
async fn test_dumpability_of_messaging_contract_loading() {
    let dump_file = UniqueAutoDeletableFile::new("dump");
    let devnet_args = ["--dump-path", &dump_file.path, "--dump-on", "exit"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let anvil = BackgroundAnvil::spawn().await.unwrap();

    devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .unwrap();

    // assert loadability while Anvil is still alive
    devnet.send_custom_rpc("devnet_dump", Value::Null).await.unwrap();
    devnet.send_custom_rpc("devnet_load", json!({ "path": dump_file.path })).await.unwrap();

    // assert loading fails if anvil not alive
    send_ctrl_c_signal_and_wait(&anvil.process).await;
    match devnet.send_custom_rpc("devnet_load", json!({ "path": dump_file.path })).await {
        Err(RpcError { message, .. }) => {
            assert_contains(&message, "error sending request for url");
            assert_contains(&message, &anvil.url);
        }
        other => panic!("Unexpected response: {other:?}"),
    };
}

#[tokio::test]
async fn flushing_only_new_messages_after_restart() {
    let anvil = BackgroundAnvil::spawn().await.unwrap();
    let (devnet, sn_account, sn_l1l2_contract) = setup_devnet(&[]).await;

    // Load l1 messaging contract.
    let load_resp = devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .unwrap();

    assert_eq!(
        load_resp.get("messaging_contract_address").unwrap().as_str().unwrap(),
        MESSAGING_L1_ADDRESS
    );

    // Deploy the L1L2 testing contract on L1 (on L2 it's already pre-deployed).
    let l1_messaging_address = H160::from_str(MESSAGING_L1_ADDRESS).unwrap();
    let eth_l1l2_address = anvil.deploy_l1l2_contract(l1_messaging_address).await.unwrap();
    let eth_l1l2_address_hex = format!("{eth_l1l2_address:#x}");

    let eth_l1l2_address_felt = Felt::from_hex_unchecked(&eth_l1l2_address_hex);
    let user_sn = Felt::ONE;
    let user_eth: U256 = 1.into();

    // Set balance to for the user 1 on L2.
    let user_balance = Felt::TWO;
    increase_balance(sn_account.clone(), sn_l1l2_contract, user_sn, user_balance).await;
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [user_balance]);

    // Withdraw on L2 to send it to L1 with a l2->l1 message.
    withdraw(sn_account.clone(), sn_l1l2_contract, user_sn, user_balance, eth_l1l2_address_felt)
        .await;
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    // Flush to send the messages.
    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();

    // Check that the balance is 0 on L1 before consuming the message.
    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, 0.into());

    let sn_l1l2_contract_u256 = felt_to_u256(sn_l1l2_contract);

    // Consume the message to increase the balance.
    anvil
        .withdraw_l1l2(
            eth_l1l2_address,
            sn_l1l2_contract_u256,
            user_eth,
            felt_to_u256(user_balance),
        )
        .await
        .unwrap();

    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, felt_to_u256(user_balance));

    // Send back the amount 1 to the user 1 on L2.
    let deposit_amount = 1;
    anvil
        .deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, deposit_amount.into())
        .await
        .unwrap();

    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, 1.into()); // 2 - 1
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    // Flush messages to have MessageToL2 executed.
    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();

    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, deposit_amount.into());
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ONE]);

    // Restart Devnet, l1-l2 messaging should be intact
    devnet.restart().await;

    // Make sure flushing doesn't process old messages
    let flush_after_restart =
        devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();
    assert_eq!(flush_after_restart["messages_to_l1"], json!([]));
    assert_eq!(flush_after_restart["messages_to_l2"], json!([]));

    // Redeploy
    let sn_l1l2_contract = deploy_l2_msg_contract(sn_account).await.unwrap();
    let sn_l1l2_contract_u256 = felt_to_u256(sn_l1l2_contract);
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    // Trigger a new action to return funds to L2
    anvil
        .deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, deposit_amount.into())
        .await
        .unwrap();

    // After depositing, there should be no funds on L1; also no funds on L2 before flushing.
    let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
    assert_eq!(user_balance_eth, 0.into()); // 2 - 1 - 1
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ZERO]);

    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();
    assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [Felt::ONE]);
}

#[tokio::test]
async fn test_getting_status_of_mock_message() {
    let (devnet, _, l1l2_contract_address) = setup_devnet(&[]).await;

    // Use postman to send a message to l2 without l1 - the message increments user balance
    let increment_amount = Felt::from(0xff);

    let user = Felt::ONE;
    let l1_tx_hash = Felt::from(0xabc);
    let mock_msg_body = json!({
        "l1_contract_address": MESSAGING_L1_ADDRESS,
        "l2_contract_address": l1l2_contract_address,
        "entry_point_selector": get_selector_from_name("deposit").unwrap(),
        "payload": [user, increment_amount],
        "paid_fee_on_l1": "0x1234",
        "nonce": "0x1",
        "l1_transaction_hash": l1_tx_hash,
    });

    let mock_msg_resp =
        devnet.send_custom_rpc("devnet_postmanSendMessageToL2", mock_msg_body).await.unwrap();
    assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [increment_amount]);

    let messages_status = devnet
        .send_custom_rpc("starknet_getMessagesStatus", json!({ "transaction_hash": l1_tx_hash }))
        .await
        .unwrap();
    assert_eq!(
        messages_status,
        json!([{
            "transaction_hash": mock_msg_resp["transaction_hash"],
            "finality_status": "ACCEPTED_ON_L2",
            "failure_reason": null,
        }])
    );
}

#[tokio::test]
async fn test_getting_status_of_real_message() {
    let anvil = BackgroundAnvil::spawn().await.unwrap();
    let (devnet, sn_account, sn_l1l2_contract) = setup_devnet(&[]).await;

    // Load l1 messaging contract.
    let body: serde_json::Value = devnet
        .send_custom_rpc("devnet_postmanLoad", json!({ "network_url": anvil.url }))
        .await
        .expect("deploy l1 messaging contract failed");

    assert_eq!(
        body.get("messaging_contract_address").unwrap().as_str().unwrap(),
        MESSAGING_L1_ADDRESS
    );

    // Deploy the L1L2 testing contract on L1 (on L2 it's already pre-deployed).
    let l1_messaging_address = H160::from_str(MESSAGING_L1_ADDRESS).unwrap();
    let eth_l1l2_address = anvil.deploy_l1l2_contract(l1_messaging_address).await.unwrap();

    let eth_l1l2_address_hex = format!("{eth_l1l2_address:#x}");
    let eth_l1l2_address_felt = Felt::from_hex_unchecked(&eth_l1l2_address_hex);

    // Set balance to 1 for the user 1 on L2 and withdraw to L1.
    let user_sn = Felt::ONE;
    let user_balance = Felt::ONE;
    increase_balance(sn_account.clone(), sn_l1l2_contract, user_sn, user_balance).await;
    withdraw(sn_account, sn_l1l2_contract, user_sn, user_balance, eth_l1l2_address_felt).await;

    // Flush to send the messages.
    devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap();

    let user_eth = 1.into();
    let sn_l1l2_contract_u256 = felt_to_u256(sn_l1l2_contract);

    // Consume the message to increase the balance on L1
    anvil.withdraw_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into()).await.unwrap();

    // Send back the amount 1 to the user 1 on L2.
    anvil.deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into()).await.unwrap();

    // Flush to trigger L2 transaction generation.
    let generated_l2_txs_raw =
        &devnet.send_custom_rpc("devnet_postmanFlush", json!({})).await.unwrap()
            ["generated_l2_transactions"];
    let generated_l2_txs = generated_l2_txs_raw.as_array().unwrap();
    assert_eq!(generated_l2_txs.len(), 1);
    let generated_l2_tx = &generated_l2_txs[0];

    let latest_l1_txs = anvil
        .provider
        .get_block(ethers::types::BlockId::Number(ethers::types::BlockNumber::Latest))
        .await
        .unwrap()
        .unwrap()
        .transactions;

    assert_eq!(latest_l1_txs.len(), 1);
    let latest_l1_tx = latest_l1_txs[0];

    let messages_status = devnet
        .send_custom_rpc("starknet_getMessagesStatus", json!({ "transaction_hash": latest_l1_tx }))
        .await
        .unwrap();
    assert_eq!(
        messages_status,
        json!([{
            "transaction_hash": generated_l2_tx,
            "finality_status": "ACCEPTED_ON_L2",
            "failure_reason": null,
        }])
    )
}
