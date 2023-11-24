pub mod common;

mod minting_tests {
    use hyper::{Body, StatusCode};
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
    };
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: &str = "0x42";
    static DUMMY_AMOUNT: u128 = 42;

    async fn increase_balance_happy_path(address: &str, init_amount: u128, mint_amount: u128) {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": address,
                "amount": mint_amount
            })
            .to_string(),
        );

        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let mut resp_body = get_json_body(resp).await;

        // tx hash is not constant so we later just assert its general form
        let tx_hash_value = resp_body["tx_hash"].take();
        assert_eq!(
            resp_body,
            json!({
                "new_balance": (init_amount + mint_amount).to_string(),
                "unit": "WEI",
                "tx_hash": null
            })
        );

        assert!(tx_hash_value.as_str().unwrap().starts_with("0x"));
    }

    #[tokio::test]
    async fn increase_balance_of_undeployed_address() {
        increase_balance_happy_path(DUMMY_ADDRESS, 0, DUMMY_AMOUNT).await;
    }

    #[tokio::test]
    async fn increase_balance_of_predeployed_account() {
        increase_balance_happy_path(
            PREDEPLOYED_ACCOUNT_ADDRESS,
            PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
            DUMMY_AMOUNT,
        )
        .await
    }

    async fn reject_bad_request(
        devnet: &BackgroundDevnet,
        json_body: serde_json::Value,
        expected_status_code: StatusCode,
    ) {
        let req_body = Body::from(json_body.to_string());
        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), expected_status_code, "Checking status of {resp:?}");
    }

    #[tokio::test]
    async fn reject_negative_amount() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        reject_bad_request(
            &devnet,
            json!({
                "address": DUMMY_ADDRESS,
                "amount": -1
            }),
            StatusCode::BAD_REQUEST,
        )
        .await;
    }

    #[tokio::test]
    async fn reject_missing_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        reject_bad_request(
            &devnet,
            json!({ "amount": DUMMY_AMOUNT }),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }

    #[tokio::test]
    async fn reject_missing_amount() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        reject_bad_request(
            &devnet,
            json!({ "address": DUMMY_ADDRESS }),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
}
