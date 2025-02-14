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
    assert_no_notifications, declare_v3_deploy_v3,
    get_events_contract_in_sierra_and_compiled_class_hash, receive_notification,
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

/// Returns deployment address.
async fn deploy_events_contract(
    account: &SingleOwnerAccount<&JsonRpcClient<HttpTransport>, LocalWallet>,
) -> Felt {
    let (sierra, casm_hash) = get_events_contract_in_sierra_and_compiled_class_hash();

    let (_, address) = declare_v3_deploy_v3(account, sierra, casm_hash, &[]).await.unwrap();
    address
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
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;
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
async fn should_notify_if_already_in_latest_block_in_on_demand_mode() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::Pending)); // for correct nonce in deployment

    let contract_address = deploy_events_contract(&account).await;
    // to have declare+deploy in one block and invoke in another
    devnet.create_block().await.unwrap();

    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    let latest_block_hash = devnet.create_block().await.unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block_hash,
            "block_number": 2, // genesis = 0, then two block creations
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_only_when_moved_from_pending_to_latest_block() {
    let devnet_args = ["--block-generation-on", "demand"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let mut account = get_single_owner_account(&devnet).await;
    account.set_block_id(BlockId::Tag(BlockTag::Pending)); // for correct nonce in deployment

    let contract_address = deploy_events_contract(&account).await;
    // to have declare+deploy in one block and invoke in another
    devnet.create_block().await.unwrap();

    let subscription_id =
        subscribe_events(&mut ws, json!({ "from_address": contract_address })).await.unwrap();

    // only receive event when pending->latest
    let invocation = emit_static_event(&account, contract_address).await.unwrap();
    assert_no_notifications(&mut ws).await;
    let latest_block_hash = devnet.create_block().await.unwrap();

    let event = receive_event(&mut ws, subscription_id).await.unwrap();
    assert_eq!(
        event,
        json!({
            "transaction_hash": invocation.transaction_hash,
            "block_hash": latest_block_hash,
            "block_number": 2, // genesis = 0, then two block creations
            "from_address": contract_address,
            "keys": [static_event_key()],
            "data": []
        })
    );

    assert_no_notifications(&mut ws).await;
}

#[tokio::test]
async fn should_notify_of_events_in_old_blocks_with_no_filters() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();

    let account = get_single_owner_account(&devnet).await;
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;

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
    let contract_address = deploy_events_contract(&account).await;

    emit_static_event(&account, contract_address).await.unwrap();
    let last_block_hash = devnet.create_block().await.unwrap();

    subscribe_events(&mut ws, json!({ "block_id": BlockId::Hash(last_block_hash) })).await.unwrap();

    assert_no_notifications(&mut ws).await;
}
