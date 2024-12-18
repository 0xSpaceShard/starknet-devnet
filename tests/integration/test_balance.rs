use serde_json::json;
use starknet_rs_core::types::Felt;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE};
use crate::common::utils::FeeUnit;

#[tokio::test]
async fn getting_balance_of_predeployed_contract() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);

    let retrieved_result =
        devnet.get_balance_latest(&contract_address, FeeUnit::WEI).await.unwrap();

    assert_eq!(retrieved_result, Felt::from(PREDEPLOYED_ACCOUNT_INITIAL_BALANCE));
}

#[tokio::test]
/// Tests the same logic that is used by BackgroundDevnet::get_balance
async fn assert_balance_endpoint_response() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    for (address, expected_balance) in [
        ("0x123", "0"), // dummy address expected to have no balance
        (PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE.to_string().as_str()),
    ] {
        for unit in ["WEI", "FRI"] {
            let json_resp: serde_json::Value = devnet
                .send_custom_rpc(
                    "devnet_getAccountBalance",
                    json!({
                        "address": address,
                        "unit": unit,
                    }),
                )
                .await
                .unwrap();

            assert_eq!(json_resp["unit"], unit);
            assert_eq!(json_resp["amount"], expected_balance);
        }
    }
}
