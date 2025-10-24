use std::collections::HashSet;

use serde_json::json;
use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, Felt, InvokeTransactionResult, TransactionFinalityStatus,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::JsonRpcClient;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_signers::LocalWallet;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, STRK_ERC20_CONTRACT_ADDRESS, UDC_CONTRACT_ADDRESS};
use crate::common::utils::{
    SubscriptionId, assert_no_notifications, declare_deploy_events_contract, receive_notification,
    receive_rpc_via_ws, subscribe, unsubscribe,
};

async fn subscribe_events(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    params: serde_json::Value,
) -> Result<SubscriptionId, anyhow::Error> {
    subscribe(ws, "starknet_subscribeEvents", params).await
}

async fn receive_event(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    subscription_id: SubscriptionId,
) -> Result<serde_json::Value, anyhow::Error> {
    receive_notification(ws, "starknet_subscriptionEvents", subscription_id).await
}

async fn get_single_owner_account(
    devnet: &BackgroundDevnet,
) -> SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet> {
    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    )
}

async fn emit_static_event(
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_address: Felt,
) -> Result<InvokeTransactionResult, anyhow::Error> {
    account
        .execute_v3(vec![Call {
            to: contract_address,
            selector: get_selector_from_name("emit_event").unwrap(),
            calldata: vec![Felt::ZERO], // what kind of event to emit
        }])
        .l1_gas(0)
        .l1_data_gas(10000)
        .l2_gas(5e7 as u64)
        .send()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

fn static_event_key() -> Felt {
    get_selector_from_name("StaticEvent").unwrap()
}

#[tokio::test]
async fn event_subscription_with_no_params_until_unsubscription() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let subscription_id = subscribe_events(&mut ws, json!({})).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();
    assert_ne!(contract_address, account.address());

    // discard notifications emitted by system contracts - asserted in a separate test
    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge
    receive_rpc_via_ws(&mut ws).await.unwrap(); // udc   - deployment
    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    let event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0, // only event in the transaction
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );

    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge
    assert_no_notifications(&mut ws).await.unwrap();

    unsubscribe(&mut ws, subscription_id).await.unwrap();

    emit_static_event(&account, contract_address).await.unwrap();
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_only_from_filtered_address() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let subscription_params = json!({ "from_address": contract_address });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_of_new_events_only_from_filtered_key() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let subscription_params = json!({ "keys": [[static_event_key()]] });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_if_already_in_latest_block_in_on_tx_mode() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();
    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_only_once_for_pre_confirmed_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws_before, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut ws_after, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::PreConfirmed)); // for correct nonce in deployment

    // Define what's needed for requests and responses
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();
    let finality_status = TransactionFinalityStatus::PreConfirmed;
    let subscription_request =
        json!({ "from_address": contract_address, "finality_status": finality_status });

    let subscription_id_before =
        subscribe_events(&mut ws_before, subscription_request.clone()).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();

    let subscription_id_after =
        subscribe_events(&mut ws_after, subscription_request).await.unwrap();

    let tx_index = devnet
        .get_pre_confirmed_block_with_tx_hashes()
        .await
        .unwrap()
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    println!("Block: {:?}", devnet.get_pre_confirmed_block_with_txs().await.unwrap());

    for (ws, subscription_id) in
        [(&mut ws_before, subscription_id_before), (&mut ws_after, subscription_id_after)]
    {
        let event = receive_event(ws, subscription_id).await.unwrap();
        assert_eq!(
            event,
            json!({
                "transaction_hash": invocation.transaction_hash,
                "transaction_index": tx_index,
                "event_index": 0,
                "from_address": contract_address,
                "keys": [static_event_key()],
                "data": [],
                "finality_status": finality_status,
            })
        );
    }

    // Should not re-notify on pre_confirmed->latest
    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws_before).await.unwrap();
    assert_no_notifications(&mut ws_after).await.unwrap();
}

#[tokio::test]
async fn should_notify_only_once_for_accepted_on_l2_in_on_demand_mode_with_explicit_tx_status() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::PreConfirmed)); // for correct nonce in deployment

    // Define what's needed for requests and responses
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    should_notify_only_once_for_accepted_on_l2_in_on_demand_mode(
        &devnet,
        &account,
        contract_address,
        json!({ "from_address": contract_address, "finality_status": TransactionFinalityStatus::AcceptedOnL2 }),
    )
    .await;
}

#[tokio::test]
async fn should_notify_only_once_for_accepted_on_l2_in_on_demand_mode_with_implicit_tx_status() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::PreConfirmed)); // for correct nonce in deployment

    // Define what's needed for requests and responses
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    should_notify_only_once_for_accepted_on_l2_in_on_demand_mode(
        &devnet,
        &account,
        contract_address,
        json!({ "from_address": contract_address }),
    )
    .await;
}

async fn should_notify_only_once_for_accepted_on_l2_in_on_demand_mode(
    devnet: &BackgroundDevnet,
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_address: Felt,
    subscription_request: serde_json::Value,
) {
    // Should work for subscriptions before and after the tx occurs.
    // Should work for explicitly and implicitly defined finality status.
    let (mut ws_before, _) = connect_async(devnet.ws_url()).await.unwrap();
    let (mut ws_after, _) = connect_async(devnet.ws_url()).await.unwrap();

    // Subscribe before tx
    let subscription_id_before =
        subscribe_events(&mut ws_before, subscription_request.clone()).await.unwrap();

    let invocation = emit_static_event(account, contract_address).await.unwrap();

    // Subscribe after tx
    let subscription_id_after =
        subscribe_events(&mut ws_after, subscription_request).await.unwrap();

    // No notifications before block creation and conversion from pre-confirmed to accepted_on_l2
    assert_no_notifications(&mut ws_before).await.unwrap();
    assert_no_notifications(&mut ws_after).await.unwrap();
    let created_block_hash = devnet.create_block().await.unwrap();

    let tx_index = devnet
        .get_latest_block_with_tx_hashes()
        .await
        .unwrap()
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    for (ws, subscription_id) in
        [(&mut ws_before, subscription_id_before), (&mut ws_after, subscription_id_after)]
    {
        let event = receive_event(ws, subscription_id).await.unwrap();
        assert_eq!(
            event,
            json!({
                "block_hash": created_block_hash,
                "transaction_index": tx_index,
                "event_index": 0,
                "block_number": 1, // the only created block
                "transaction_hash": invocation.transaction_hash,
                "from_address": contract_address,
                "keys": [static_event_key()],
                "data": [],
                "finality_status": TransactionFinalityStatus::AcceptedOnL2,
            })
        );
    }

    // Should not re-notify on next block
    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws_before).await.unwrap();
    assert_no_notifications(&mut ws_after).await.unwrap();
}

#[tokio::test]
async fn should_notify_of_events_in_old_blocks_with_no_filters() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == invocation.transaction_hash)
        .unwrap();

    // The declaration happens at block_number=1 so we query from there to latest
    let subscription_params = json!({ "block_id": BlockId::Number(1) });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // declaration of events contract fee charge
    let declaration_fee_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(declaration_fee_event["block_number"], 1);
    assert_eq!(declaration_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS));

    // deployment of events contract: udc invocation
    let deployment_udc_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(deployment_udc_event["block_number"], 2);
    assert_eq!(deployment_udc_event["from_address"], json!(UDC_CONTRACT_ADDRESS));

    // deployment of events contract: fee charge
    let deployment_fee_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(deployment_fee_event["block_number"], 2);
    assert_eq!(deployment_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS),);

    // invocation of events contract
    let invocation_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );

    // invocation of events contract fee charge
    let invocation_fee_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(invocation_fee_event["block_number"], latest_block.block_number);
    assert_eq!(invocation_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS));

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_of_old_and_new_events_with_address_filter() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let old_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let block_before_subscription = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = block_before_subscription
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == old_invocation.transaction_hash)
        .unwrap();

    // The declaration happens at block_number=1, but only invocation should be notified of
    let subscription_params =
        json!({ "block_id": BlockId::Number(1), "from_address": contract_address });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // assert presence of old event (event that was triggered before the subscription)
    let old_invocation_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        old_invocation_event,
        json!({
            "transaction_hash": old_invocation.transaction_hash,
            "block_hash": block_before_subscription.block_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_number": block_before_subscription.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );
    assert_no_notifications(&mut ws).await.unwrap();

    // new event (after subscription)
    let new_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == new_invocation.transaction_hash)
        .unwrap();
    let new_invocation_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        new_invocation_event,
        json!({
            "transaction_hash": new_invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_of_old_and_new_events_with_key_filter() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let old_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let block_before_subscription = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = block_before_subscription
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == old_invocation.transaction_hash)
        .unwrap();

    // The declaration happens at block_number=1, but only invocation should be notified of
    let subscription_params =
        json!({ "block_id": BlockId::Number(1), "keys": [[static_event_key()]] });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // assert presence of old event (event that was triggered before the subscription)
    let invocation_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": old_invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": block_before_subscription.block_hash,
            "block_number": block_before_subscription.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );
    assert_no_notifications(&mut ws).await.unwrap();

    // new event (after subscription)
    let new_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let tx_index = latest_block
        .transactions
        .iter()
        .position(|tx_hash| *tx_hash == new_invocation.transaction_hash)
        .unwrap();
    let invocation_event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": new_invocation.transaction_hash,
            "transaction_index": tx_index,
            "event_index": 0,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
            "finality_status": TransactionFinalityStatus::AcceptedOnL2,
        })
    );
    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_not_notify_of_events_in_too_old_blocks() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    emit_static_event(&account, contract_address).await.unwrap();
    let last_block_hash = devnet.create_block().await.unwrap();

    subscribe_events(&mut ws, json!({ "block_id": BlockId::Hash(last_block_hash) })).await.unwrap();

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn should_notify_of_events_in_old_blocks() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let block_number_before_emission =
        devnet.get_latest_block_with_tx_hashes().await.unwrap().block_number;

    let mut txs_with_invocation_events = HashSet::new();
    let expected_events = 3;
    for _ in 0..expected_events {
        let invocation_event = emit_static_event(&account, contract_address).await.unwrap();
        txs_with_invocation_events.insert(invocation_event.transaction_hash);
    }

    let subscription_id = subscribe_events(
        &mut ws,
        json!({ "block_id": BlockId::Number(0), "from_address": contract_address }),
    )
    .await
    .unwrap();

    let mut received_tx_hashes_from_events = HashSet::new();
    for i in 0..expected_events {
        let event_notification = receive_event(&mut ws, subscription_id.clone()).await.unwrap();
        let tx_hash =
            Felt::from_hex_unchecked(event_notification["transaction_hash"].as_str().unwrap());
        received_tx_hashes_from_events.insert(tx_hash);

        assert_eq!(event_notification["block_number"], json!(block_number_before_emission + i + 1));
        assert_eq!(event_notification["from_address"], json!(contract_address));
        assert_eq!(event_notification["keys"], json!([static_event_key()]));
        assert_eq!(event_notification["data"], json!([]));
    }

    assert_eq!(received_tx_hashes_from_events, txs_with_invocation_events);

    assert_no_notifications(&mut ws).await.unwrap();
}

#[tokio::test]
async fn test_fork_subscription_to_events() {
    // Setup original devnet
    let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

    // Deploy event contract on origin
    let account = get_single_owner_account(&origin_devnet).await;
    let event_contract_address = declare_deploy_events_contract(&account).await.unwrap();

    // Emit static events on origin
    let n_origin_events = 3;
    let mut origin_tx_hashes = Vec::new();
    for _ in 0..n_origin_events {
        let invocation = emit_static_event(&account, event_contract_address).await.unwrap();
        origin_tx_hashes.push(invocation.transaction_hash);
        origin_devnet.create_block().await.unwrap();
    }

    // Fork the devnet
    let fork_devnet = origin_devnet.fork().await.unwrap();

    // Create some blocks on fork devnet before subscribing (to test empty block handling)
    fork_devnet.create_block().await.unwrap();

    // Subscribe to events on the forked devnet
    let (mut ws, _) = connect_async(fork_devnet.ws_url()).await.unwrap();
    let subscription_id = subscribe_events(
        &mut ws,
        json!({
            "block_id": BlockId::Number(0),
            "from_address": event_contract_address.to_hex_string()
        }),
    )
    .await
    .unwrap();

    // We should receive notifications for all events from origin
    for hash in origin_tx_hashes.iter() {
        let event = receive_event(&mut ws, subscription_id.clone()).await.unwrap();

        // Verify event data
        assert_eq!(event["from_address"], json!(event_contract_address));
        assert_eq!(event["keys"][0], json!(static_event_key()));
        assert_eq!(event["data"], json!([]));
        assert_eq!(event["transaction_hash"], json!(hash));
    }

    // Emit additional events on the forked devnet

    let (wallet, address) = origin_devnet.get_first_predeployed_account().await; // to avoid nonce clash
    let original_account_on_fork = SingleOwnerAccount::new(
        &fork_devnet.json_rpc_client,
        wallet,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let n_fork_events = 2;
    for _ in 0..n_fork_events {
        let invocation =
            emit_static_event(&original_account_on_fork, event_contract_address).await.unwrap();
        // Verify we get notification for the new event
        let event = receive_event(&mut ws, subscription_id.clone()).await;

        let event = event.unwrap();
        assert_eq!(event["from_address"], json!(event_contract_address));
        assert_eq!(event["keys"][0], json!(static_event_key()));
        assert_eq!(event["data"], json!([]));
        assert_eq!(event["transaction_hash"], json!(invocation.transaction_hash));
    }

    assert_no_notifications(&mut ws).await.unwrap();
}
