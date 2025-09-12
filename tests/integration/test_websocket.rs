use std::sync::Arc;

use serde_json::json;
use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{
    BroadcastedDeclareTransactionV3, DataAvailabilityMode, Felt, Transaction,
};
use starknet_rs_signers::Signer;
use tokio_tungstenite::connect_async;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::{
    FeeUnit, LocalFee, UniqueAutoDeletableFile, assert_no_notifications,
    get_simple_contract_artifacts, send_binary_rpc_via_ws, send_ctrl_c_signal_and_wait,
    send_text_rpc_via_ws, subscribe,
};

#[tokio::test]
async fn mint_and_check_tx_via_websocket() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mint_resp = send_text_rpc_via_ws(
        &mut ws,
        "devnet_mint",
        json!({ "address": "0x1", "amount": 100, "unit": "FRI" }),
    )
    .await
    .unwrap();

    let tx_hash = Felt::from_hex_unchecked(mint_resp["result"]["tx_hash"].as_str().unwrap());

    let tx_raw = send_text_rpc_via_ws(
        &mut ws,
        "starknet_getTransactionByHash",
        json!({ "transaction_hash": tx_hash }),
    )
    .await
    .unwrap();

    let tx: Transaction = serde_json::from_value(tx_raw["result"].clone()).unwrap();
    assert_eq!(tx.transaction_hash(), &tx_hash);
}

#[tokio::test]
async fn create_block_via_binary_ws_message() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let block_specifier = json!({ "block_id": "latest" });
    let block_resp_before =
        send_binary_rpc_via_ws(&mut ws, "starknet_getBlockWithTxs", block_specifier.clone())
            .await
            .unwrap();
    assert_eq!(block_resp_before["result"]["block_number"], 0);

    let creation_resp =
        send_binary_rpc_via_ws(&mut ws, "devnet_createBlock", json!({})).await.unwrap();
    assert!(creation_resp["result"].is_object());

    let block_resp_after =
        send_binary_rpc_via_ws(&mut ws, "starknet_getBlockWithTxs", block_specifier).await.unwrap();
    assert_eq!(block_resp_after["result"]["block_number"], 1);
}

#[tokio::test]
async fn multiple_ws_connections() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let iterations = 10;

    let mut ws_streams = vec![];
    for _ in 0..iterations {
        let (ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        ws_streams.push(ws);
    }

    let dummy_address = Felt::ONE;
    let single_mint_amount = 11;
    let mint_params = json!({ "address": dummy_address, "amount": single_mint_amount });
    for ws in &mut ws_streams {
        send_text_rpc_via_ws(ws, "devnet_mint", mint_params.clone()).await.unwrap();
    }

    let balance = devnet.get_balance_latest(&dummy_address, FeeUnit::Fri).await.unwrap();
    assert_eq!(balance, Felt::from(single_mint_amount * iterations));
}

#[tokio::test]
async fn invalid_general_rpc_request() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let resp = send_text_rpc_via_ws(&mut ws, "devnet_mint", json!({})).await.unwrap();
    assert_eq!(resp["error"]["message"], "missing field `address`");
}

#[tokio::test]
async fn restarting_should_forget_all_websocket_subscriptions() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    devnet.create_block().await.unwrap();

    subscribe(&mut ws, "starknet_subscribeNewHeads", json!({})).await.unwrap();

    devnet.restart().await;

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn test_invalid_rpc_methods_via_ws() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    for method in [
        "starknet_invalid",
        "definitely_invalid",
        "devnet_invalid",
        "starknet_subscribeInvalid",
        "starknet_unsubscribeInvalid",
    ] {
        let resp = send_text_rpc_via_ws(&mut ws, method, json!({})).await.unwrap();
        assert_eq!(
            resp,
            json!({
                "jsonrpc": "2.0",
                "id": 0,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                }
            })
        );
    }
}

#[tokio::test]
async fn method_restriction_applies_via_ws() {
    let devnet_args = ["--restrictive-mode", "devnet_mint"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mint_resp = send_text_rpc_via_ws(
        &mut ws,
        "devnet_mint",
        json!({ "address": "0x1", "amount": 100, "unit": "FRI" }),
    )
    .await
    .unwrap();

    assert_eq!(
        mint_resp,
        json!({"jsonrpc":"2.0","id":0,"error":{"code":-32604,"message":"Method forbidden"}})
    )
}

#[tokio::test]
async fn should_load_correct_devnet_with_state_modified_via_ws() {
    let devnet_args = ["--dump-on", "request"];
    let devnet_dumpable = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws_on_dumped, _) = connect_async(devnet_dumpable.ws_url()).await.unwrap();

    let mint_address = "0x1";
    let mint_amount = 100;
    send_text_rpc_via_ws(
        &mut ws_on_dumped,
        "devnet_mint",
        json!({ "address": mint_address, "amount": mint_amount, "unit": "FRI" }),
    )
    .await
    .unwrap();

    let dump_file = UniqueAutoDeletableFile::new("tmp");

    let dump_resp =
        send_text_rpc_via_ws(&mut ws_on_dumped, "devnet_dump", json!({ "path": dump_file.path }))
            .await
            .unwrap();
    assert_eq!(dump_resp, json!({ "jsonrpc": "2.0", "id": 0, "result": null }));

    drop(ws_on_dumped);
    send_ctrl_c_signal_and_wait(&devnet_dumpable.process).await;

    let devnet_loaded = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws_on_loaded, _) = connect_async(devnet_loaded.ws_url()).await.unwrap();

    let load_resp =
        send_text_rpc_via_ws(&mut ws_on_loaded, "devnet_load", json!({ "path": dump_file.path }))
            .await
            .unwrap();
    assert_eq!(load_resp, json!({ "jsonrpc": "2.0", "id": 0, "result": {} }));

    let balance_resp = send_text_rpc_via_ws(
        &mut ws_on_loaded,
        "devnet_getAccountBalance",
        json!({ "address": mint_address, "unit": "FRI" }),
    )
    .await
    .unwrap();

    assert_eq!(
        balance_resp,
        json!({ "jsonrpc": "2.0", "id": 0, "result": {"amount": "100", "unit": "FRI"} })
    );
}

#[tokio::test]
async fn should_support_restarting_via_ws() {
    let devnet_args = ["--dump-on", "request"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mint_address = "0x1";
    let mint_amount = 100;
    send_text_rpc_via_ws(
        &mut ws,
        "devnet_mint",
        json!({ "address": mint_address, "amount": mint_amount, "unit": "FRI" }),
    )
    .await
    .unwrap();

    let restart_resp = send_text_rpc_via_ws(&mut ws, "devnet_restart", json!({})).await.unwrap();
    assert_eq!(restart_resp, json!({ "jsonrpc": "2.0", "id": 0, "result": {} }));

    let balance_resp = send_text_rpc_via_ws(
        &mut ws,
        "devnet_getAccountBalance",
        json!({ "address": mint_address, "unit": "FRI" }),
    )
    .await
    .unwrap();

    assert_eq!(
        balance_resp,
        json!({ "jsonrpc": "2.0", "id": 0, "result": {"amount": "0", "unit": "FRI"} })
    );
}

#[tokio::test]
async fn should_declare_via_ws() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    // Prepare account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    // Prepare class
    let (simple_class, casm_hash) = get_simple_contract_artifacts();
    let simple_class = Arc::new(simple_class);

    // Prepare declaration - make a signable instance
    let declaration = account.declare_v3(simple_class.clone(), casm_hash);
    let nonce = Felt::ZERO;
    let fee = LocalFee::from(declaration.estimate_fee().await.unwrap());
    let signable_declaration = declaration
        .l1_gas(fee.l1_gas)
        .l1_gas_price(fee.l1_gas_price)
        .l2_gas(fee.l2_gas)
        .l2_gas_price(fee.l2_gas_price)
        .l1_data_gas(fee.l1_data_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .nonce(nonce)
        .prepared()
        .unwrap();

    // Prepare declaration - sign it
    let declaration_hash = signable_declaration.transaction_hash(false);
    let signature = signer.sign_hash(&declaration_hash).await.unwrap();

    // Send the declaration tx via ws
    let sendable_declaration = BroadcastedDeclareTransactionV3 {
        sender_address: account_address,
        compiled_class_hash: casm_hash,
        signature: vec![signature.r, signature.s],
        nonce,
        contract_class: simple_class,
        resource_bounds: fee.into(),
        tip: 0,
        paymaster_data: vec![],
        account_deployment_data: vec![],
        nonce_data_availability_mode: DataAvailabilityMode::L1,
        fee_data_availability_mode: DataAvailabilityMode::L1,
        is_query: false,
    };

    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    let declaration_resp = send_text_rpc_via_ws(
        &mut ws,
        "starknet_addDeclareTransaction",
        json!({ "declare_transaction": sendable_declaration }),
    )
    .await
    .unwrap();

    let received_declaration_hash =
        Felt::from_hex_unchecked(declaration_resp["result"]["transaction_hash"].as_str().unwrap());
    assert_eq!(declaration_hash, received_declaration_hash);

    // Assert sending the tx was successful
    let tx_status = send_text_rpc_via_ws(
        &mut ws,
        "starknet_getTransactionStatus",
        json!({ "transaction_hash": declaration_hash }),
    )
    .await
    .unwrap();

    assert_eq!(
        tx_status["result"],
        json!({
            "finality_status": "ACCEPTED_ON_L2",
            "failure_reason": null,
            "execution_status": "SUCCEEDED",
        })
    );
}
