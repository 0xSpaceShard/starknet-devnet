use serde_json::json;
use starknet_rs_core::types::Felt;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE};
use crate::common::utils::FeeUnit;

static DUMMY_ADDRESS: &str = "0x42";
static DUMMY_AMOUNT: u128 = 42;

async fn increase_balance_happy_path(
    address: &str,
    init_amount: u128,
    mint_amount: u128,
    unit: FeeUnit,
) {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let mut resp_body: serde_json::Value = devnet
        .send_custom_rpc(
            "devnet_mint",
            json!({
                "address": address,
                "amount": mint_amount,
                "unit": unit,
            }),
        )
        .await
        .unwrap();

    // tx hash is not constant so we just assert its general form
    let tx_hash_value = resp_body["tx_hash"].take();
    assert!(tx_hash_value.as_str().unwrap().starts_with("0x"));

    let final_balance = Felt::from(init_amount) + Felt::from(mint_amount);
    assert_eq!(
        resp_body,
        json!({
            "new_balance": final_balance.to_biguint().to_string(),
            "unit": unit,
            "tx_hash": null
        })
    );

    let new_balance =
        devnet.get_balance_latest(&Felt::from_hex_unchecked(address), unit).await.unwrap();
    assert_eq!(final_balance, new_balance);
}

#[tokio::test]
async fn increase_balance_of_undeployed_address_wei() {
    increase_balance_happy_path(DUMMY_ADDRESS, 0, DUMMY_AMOUNT, FeeUnit::Wei).await;
}

#[tokio::test]
async fn increase_balance_of_undeployed_address_fri() {
    increase_balance_happy_path(DUMMY_ADDRESS, 0, DUMMY_AMOUNT, FeeUnit::Fri).await;
}

#[tokio::test]
async fn increase_balance_of_undeployed_address_unit_not_specified() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let unit_not_specified = json!({ "address": DUMMY_ADDRESS, "amount": DUMMY_AMOUNT });
    let mut resp_body: serde_json::Value =
        devnet.send_custom_rpc("devnet_mint", unit_not_specified).await.unwrap();

    let tx_hash_value = resp_body["tx_hash"].take();

    // tx hash is not constant so we later just assert its general form
    assert_eq!(
        resp_body,
        json!({
            "new_balance": DUMMY_AMOUNT.to_string(),
            "unit": "FRI",
            "tx_hash": null
        })
    );

    assert!(tx_hash_value.as_str().unwrap().starts_with("0x"));
}

#[tokio::test]
async fn increase_balance_of_predeployed_account() {
    increase_balance_happy_path(
        PREDEPLOYED_ACCOUNT_ADDRESS,
        PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
        DUMMY_AMOUNT,
        FeeUnit::Wei,
    )
    .await
}

#[tokio::test]
async fn increase_balance_of_predeployed_account_u256() {
    increase_balance_happy_path(
        PREDEPLOYED_ACCOUNT_ADDRESS,
        PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
        u128::MAX,
        FeeUnit::Wei,
    )
    .await
}

#[tokio::test]
#[ignore = "Currently, starknet_rs_core::types::BroadcastedDeclareTransaction::V3 is not \
            implemented so once it is available we could add test like this"]
async fn execute_v3_transaction_with_strk_token() {
    // 1. run BackgroundDevnet
    // 2. try sending declare v3 tx - expect failure
    // 3. mint some STRK to the account (keeping WEI at 0)
    // 4. now tx succeeds
}

async fn reject_bad_minting_request(json_body: serde_json::Value) {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let rpc_error = devnet.send_custom_rpc("devnet_mint", json_body).await.unwrap_err();
    assert_eq!(rpc_error.code, -32602, "Checking status of {rpc_error:?}");
}

#[tokio::test]
async fn reject_unknown_unit() {
    reject_bad_minting_request(json!({
        "address": DUMMY_ADDRESS,
        "amount": DUMMY_AMOUNT,
        "unit": "Satoshi"
    }))
    .await;
}

#[tokio::test]
async fn reject_negative_amount() {
    reject_bad_minting_request(json!({ "address": DUMMY_ADDRESS, "amount": -1 })).await;
}

#[tokio::test]
async fn reject_missing_address() {
    reject_bad_minting_request(json!({ "amount": DUMMY_AMOUNT })).await;
}

#[tokio::test]
async fn reject_missing_amount() {
    reject_bad_minting_request(json!({ "address": DUMMY_ADDRESS })).await;
}

async fn reject_bad_json_rpc_request(devnet: &BackgroundDevnet, body: serde_json::Value) {
    let rpc_error = devnet.send_custom_rpc("devnet_getAccountBalance", body).await.unwrap_err();
    assert_eq!(rpc_error.code, -32602);
}

#[tokio::test]
async fn reject_if_no_params_when_querying() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    reject_bad_json_rpc_request(&devnet, json!({})).await;
}

#[tokio::test]
async fn reject_missing_address_when_querying() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    reject_bad_json_rpc_request(&devnet, json!({ "unit": "FRI" })).await;
}

#[tokio::test]
async fn reject_invalid_unit_when_querying() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    reject_bad_json_rpc_request(&devnet, json!({ "address": "0x1", "unit": "INVALID" })).await;
}

#[tokio::test]
async fn test_overflow_behavior() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let mint_err = devnet
        .send_custom_rpc(
            "devnet_mint",
            serde_json::from_str(r#"{{
                "address": "0x1",
                "amount" : 72370055773322622139731865630429942408293740416025352524660990004945706024960, 
                "unit": "FRI"
            }}"#).unwrap()
        )
        .await
        .unwrap_err();
    assert_eq!(
        (
            mint_err.code,
            mint_err.message,
            mint_err.data.unwrap()["revert_reason"].as_str().unwrap()
        ),
        (
            -1,
            "Minting reverted".into(),
            "The requested minting amount overflows the token contract's total_supply."
        )
    );
}
