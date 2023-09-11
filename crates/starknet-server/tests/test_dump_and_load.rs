pub mod common;

mod dump_and_load_tests {
    use std::thread;
    use std::time::Duration;

    use hyper::{Body, StatusCode};
    use serde_json::json;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: &str = "0x42";
    static DUMMY_AMOUNT: u128 = 42;

    #[tokio::test]
    async fn mint() {
        let mut devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": DUMMY_ADDRESS,
                "amount": DUMMY_AMOUNT
            })
            .to_string(),
        );
        let resp = devnet_dump.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let mut resp_body = get_json_body(resp).await;
        let tx_hash_value = resp_body["tx_hash"].take();
        println!("devnet_dump id: {:?}", devnet_dump.process.id());
        // devnet_dump.process.kill();

        devnet_dump.process.kill().unwrap();
        thread::sleep(Duration::from_secs(5));
        devnet_dump.process.kill().expect("Failed to kill child process");

        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        println!("devnet_load id: {:?}", devnet_dump.process.id());
        println!("get tx of tx_hash_value: {:?}", tx_hash_value);
    }
}
