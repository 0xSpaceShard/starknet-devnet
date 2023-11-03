pub mod common;

mod advancing_time_tests {

    use std::{thread, time};

    use hyper::Body;
    use serde_json::json;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{get_json_body, get_unix_timestamp_as_seconds};

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn set_time_in_past() {
        // set time and assert
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "time": past_time
            })
            .to_string(),
        );
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], past_time);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(set_time_block["timestamp"].as_u64() >= Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than past_time
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(empty_block["timestamp"].as_u64() > Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(mint_block["timestamp"].as_u64() > empty_block["block_timestamp"].as_u64());
    }

    #[tokio::test]
    async fn set_time_in_future() {
        // set time and assert
        let now = get_unix_timestamp_as_seconds();
        let future_time = now + 100;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "time": future_time
            })
            .to_string(),
        );
        let resp = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body = get_json_body(resp).await;
        assert_eq!(resp_body["block_timestamp"], future_time);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(set_time_block["timestamp"].as_u64() >= Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than future_time
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(empty_block["timestamp"].as_u64() > Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(mint_block["timestamp"].as_u64() > empty_block["timestamp"].as_u64());
    }

    #[tokio::test]
    async fn set_time_empty_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(json!({}).to_string());
        let result = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn set_time_wrong_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "test": 0
            })
            .to_string(),
        );
        let result = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn increase_time() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let now = get_unix_timestamp_as_seconds();

        // increase time and assert if it's greater than now
        let first_increase_time: u64 = 1000;
        let first_increase_time_body = Body::from(
            json!({
                "time": first_increase_time
            })
            .to_string(),
        );
        devnet.post_json("/increase_time".into(), first_increase_time_body).await.unwrap();
        let first_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(first_increase_time_block["timestamp"].as_u64() >= Some(now + first_increase_time));

        // second increase time
        let second_increase_time: u64 = 100;
        let second_increase_time_body = Body::from(
            json!({
                "time": second_increase_time
            })
            .to_string(),
        );
        devnet.post_json("/increase_time".into(), second_increase_time_body).await.unwrap();
        let second_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(
            second_increase_time_block["timestamp"].as_u64()
                >= Some(now + first_increase_time + second_increase_time)
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check again if block_timestamp is greater than last block
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(
            empty_block["timestamp"].as_u64() > second_increase_time_block["timestamp"].as_u64()
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert!(last_block["timestamp"].as_u64() > empty_block["timestamp"].as_u64());
    }

    #[tokio::test]
    async fn increase_time_empty_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let increase_time_body = Body::from(json!({}).to_string());
        let result = devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn increase_time_wrong_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let increase_time_body = Body::from(
            json!({
                "test": 0
            })
            .to_string(),
        );
        let result = devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }
}
