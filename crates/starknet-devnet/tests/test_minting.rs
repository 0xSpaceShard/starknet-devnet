pub mod common;

mod minting_tests {
    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_rs_core::types::FieldElement;
    use starknet_types::num_bigint::BigUint;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
    };
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: &str = "0x42";
    static DUMMY_AMOUNT: u128 = 42;

    async fn increase_balance_happy_path(
        address: &str,
        init_amount: u128,
        mint_amount: u128,
        unit: FeeUnit,
    ) {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": address,
                "amount": mint_amount,
                "unit": unit,
            })
            .to_string(),
        );

        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let mut resp_body = get_json_body(resp).await;

        // tx hash is not constant so we later just assert its general form
        let tx_hash_value = resp_body["tx_hash"].take();
        let final_balance = BigUint::from(init_amount) + BigUint::from(mint_amount);
        assert_eq!(
            resp_body,
            json!({
                "new_balance": final_balance.to_string(),
                "unit": unit,
                "tx_hash": null
            })
        );

        assert!(tx_hash_value.as_str().unwrap().starts_with("0x"));

        let new_balance =
            devnet.get_balance(&FieldElement::from_hex_be(address).unwrap(), unit).await.unwrap();

        let final_balance = FieldElement::from_dec_str(&final_balance.to_str_radix(10)).unwrap();
        assert_eq!(final_balance, new_balance);
    }

    #[tokio::test]
    async fn increase_balance_of_undeployed_address_wei() {
        increase_balance_happy_path(DUMMY_ADDRESS, 0, DUMMY_AMOUNT, FeeUnit::WEI).await;
    }

    #[tokio::test]
    async fn increase_balance_of_undeployed_address_fri() {
        increase_balance_happy_path(DUMMY_ADDRESS, 0, DUMMY_AMOUNT, FeeUnit::FRI).await;
    }

    #[tokio::test]
    async fn increase_balance_of_undeployed_address_unit_not_specified() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let unit_not_specified = json!({
            "address": DUMMY_ADDRESS,
            "amount": DUMMY_AMOUNT,
        });

        let resp = devnet
            .post_json("/mint".into(), Body::from(unit_not_specified.to_string()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let mut resp_body = get_json_body(resp).await;
        let tx_hash_value = resp_body["tx_hash"].take();

        // tx hash is not constant so we later just assert its general form
        assert_eq!(
            resp_body,
            json!({
                "new_balance": DUMMY_AMOUNT.to_string(),
                "unit": "WEI",
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
            FeeUnit::WEI,
        )
        .await
    }

    #[tokio::test]
    async fn increase_balance_of_predeployed_account_u256() {
        increase_balance_happy_path(
            PREDEPLOYED_ACCOUNT_ADDRESS,
            PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
            u128::MAX,
            FeeUnit::WEI,
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
        let req_body = Body::from(json_body.to_string());
        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "Checking status of {resp:?}");
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
        reject_bad_minting_request(json!({
            "address": DUMMY_ADDRESS,
            "amount": -1
        }))
        .await;
    }

    #[tokio::test]
    async fn reject_missing_address() {
        reject_bad_minting_request(json!({ "amount": DUMMY_AMOUNT })).await;
    }

    #[tokio::test]
    async fn reject_missing_amount() {
        reject_bad_minting_request(json!({ "address": DUMMY_ADDRESS })).await;
    }

    async fn reject_bad_balance_query(query: &str) {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.get("/account_balance", Some(query.into())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "Checking status of {resp:?}");
    }

    #[tokio::test]
    async fn reject_if_no_params_when_querying() {
        reject_bad_balance_query("").await;
    }

    #[tokio::test]
    async fn reject_missing_address_when_querying() {
        reject_bad_balance_query("unit=FRI").await;
    }

    #[tokio::test]
    async fn reject_invalid_unit_when_querying() {
        reject_bad_balance_query("address=0x1&unit=INVALID").await;
    }
}
