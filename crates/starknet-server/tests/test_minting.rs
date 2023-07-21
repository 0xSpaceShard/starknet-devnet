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
        assert_eq!(resp.status(), StatusCode::OK);

        let resp_body = resp.into_body();
        let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
        let deserialized_resp_body: serde_json::Value =
            serde_json::from_slice(&resp_body_bytes).unwrap();
        assert_eq!(
            deserialized_resp_body,
            json!({
                "new_balance": DUMMY_AMOUNT.to_string(),
                "unit": "WEI",
                "tx_hash": "0x123" // TODO will not work - perhaps ignore the tx hash
            })
        );
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
