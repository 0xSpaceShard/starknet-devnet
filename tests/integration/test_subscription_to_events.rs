use std::collections::HashSet;

use serde_json::json;
use starknet_core::constants::{STRK_ERC20_CONTRACT_ADDRESS, UDC_CONTRACT_ADDRESS};
use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::types::{BlockId, BlockTag, Call, Felt, InvokeTransactionResult};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::JsonRpcClient;
use starknet_rs_signers::LocalWallet;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants;
use crate::common::utils::{
    assert_no_notifications, declare_deploy_events_contract, receive_notification,
    receive_rpc_via_ws, subscribe, unsubscribe, SubscriptionId,
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
    // precalculated
    let invocation_fee = 15_000_000_000_000_u64;
    let gas_price = 100_000_000_000_u64;
    account
        .execute_v3(vec![Call {
            to: contract_address,
            selector: get_selector_from_name("emit_event").unwrap(),
            calldata: vec![Felt::ZERO], // what kind of event to emit
        }])
        .gas(invocation_fee / gas_price)
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

    // discard notifications emitted by system contracts - asserted in a separate test
    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge
    receive_rpc_via_ws(&mut ws).await.unwrap(); // udc   - deployment
    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    receive_rpc_via_ws(&mut ws).await.unwrap(); // erc20 - fee charge
    assert_no_notifications(&mut ws).await;

    unsubscribe(&mut ws, subscription_id).await.unwrap();

    emit_static_event(&account, contract_address).await.unwrap();
    assert_no_notifications(&mut ws).await;
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

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    assert_no_notifications(&mut ws).await;
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

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_if_already_in_latest_block_in_on_tx_mode() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();
    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_only_once_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::Pending)); // for correct nonce in deployment

    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": [],
        })
    );

    // should not renotify on pending->latest
    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_not_notify_again_when_pending_becomes_latest() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::Pending)); // for correct nonce in deployment

    let contract_address = declare_deploy_events_contract(&account).await.unwrap();
    // to have declare+deploy in one block and invoke in another
    devnet.create_block().await.unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    // event notification should be received immediately
    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    devnet.create_block().await.unwrap();
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_events_in_old_blocks_with_no_filters() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    // The declaration happens at block_number=1 so we query from there to latest
    let subscription_params = json!({ "block_id": BlockId::Number(1) });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // declaration of events contract fee charge
    let declaration_fee_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(declaration_fee_event["block_number"], 1);
    assert_eq!(declaration_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS));

    // deployment of events contract: udc invocation
    let deployment_udc_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(deployment_udc_event["block_number"], 2);
    assert_eq!(deployment_udc_event["from_address"], json!(UDC_CONTRACT_ADDRESS));

    // deployment of events contract: fee charge
    let deployment_fee_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(deployment_fee_event["block_number"], 2);
    assert_eq!(deployment_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS),);

    // invocation of events contract
    let invocation_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    // invocation of events contract fee charge
    let invocation_fee_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(invocation_fee_event["block_number"], latest_block.block_number);
    assert_eq!(invocation_fee_event["from_address"], json!(STRK_ERC20_CONTRACT_ADDRESS));

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_old_and_new_events_with_address_filter() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let old_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let block_before_subscription = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    // The declaration happens at block_number=1, but only invocation should be notified of
    let subscription_params =
        json!({ "block_id": BlockId::Number(1), "from_address": contract_address });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // assert presence of old event (event that was triggered before the subscription)
    let old_invocation_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        old_invocation_event,
        json!({
            "transaction_hash": old_invocation.transaction_hash,
            "block_hash": block_before_subscription.block_hash,
            "block_number": block_before_subscription.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );
    assert_no_notifications(&mut ws).await;

    // new event (after subscription)
    let new_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let new_invocation_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        new_invocation_event,
        json!({
            "transaction_hash": new_invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );
    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_old_and_new_events_with_key_filter() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = declare_deploy_events_contract(&account).await.unwrap();

    let old_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let block_before_subscription = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    // The declaration happens at block_number=1, but only invocation should be notified of
    let subscription_params =
        json!({ "block_id": BlockId::Number(1), "keys": [[static_event_key()]] });
    let subscription_id = subscribe_events(&mut ws, subscription_params).await.unwrap();

    // assert presence of old event (event that was triggered before the subscription)
    let invocation_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": old_invocation.transaction_hash,
            "block_hash": block_before_subscription.block_hash,
            "block_number": block_before_subscription.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );
    assert_no_notifications(&mut ws).await;

    // new event (after subscription)
    let new_invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
    let invocation_event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        invocation_event,
        json!({
            "transaction_hash": new_invocation.transaction_hash,
            "block_hash": latest_block.block_hash,
            "block_number": latest_block.block_number,
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );
    assert_no_notifications(&mut ws).await;
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

    assert_no_notifications(&mut ws).await;
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
        let event_notification = receive_event(&mut ws, subscription_id).await.unwrap();
        let tx_hash =
            Felt::from_hex_unchecked(event_notification["transaction_hash"].as_str().unwrap());
        received_tx_hashes_from_events.insert(tx_hash);

        assert_eq!(event_notification["block_number"], json!(block_number_before_emission + i + 1));
        assert_eq!(event_notification["from_address"], json!(contract_address));
        assert_eq!(event_notification["keys"], json!([static_event_key()]));
        assert_eq!(event_notification["data"], json!([]));
    }

    assert_eq!(received_tx_hashes_from_events, txs_with_invocation_events);

    assert_no_notifications(&mut ws).await;
}
