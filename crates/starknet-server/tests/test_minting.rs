pub mod common;

mod minting_tests {
    use hyper::{Body, StatusCode};
    use serde_json::json;

    use crate::common::util::BackgroundDevnet;

    static DUMMY_ADDRESS: &str = "0x42";
    static DUMMY_AMOUNT: u32 = 42;

    #[tokio::test]
    async fn increase_balance_of_undeployed_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": DUMMY_ADDRESS,
                "amount": DUMMY_AMOUNT
            })
            .to_string(),
        );

        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let resp_body = resp.into_body();
        let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
        let mut deserialized_resp_body: serde_json::Value =
            serde_json::from_slice(&resp_body_bytes).unwrap();

        // tx hash is not constant so we later just assert its general form
        let tx_hash_value = deserialized_resp_body["tx_hash"].take();
        assert_eq!(
            deserialized_resp_body,
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
        todo!();
    }

    #[tokio::test]
    async fn reject_negative_amount() {
        todo!();
    }
}
