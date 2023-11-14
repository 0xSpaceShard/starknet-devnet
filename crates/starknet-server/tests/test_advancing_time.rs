pub mod common;

mod advancing_time_tests {

    use std::{thread, time};

    use hyper::Body;
    use serde_json::json;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{get_json_body, get_unix_timestamp_as_seconds};

    const DUMMY_ADDRESS: u128 = 1;
    const DUMMY_AMOUNT: u128 = 1;
    const BUFFER_TIME_SECONDS: u64 = 5;

    pub fn assert_ge_with_buffer(val1: Option<u64>, val2: Option<u64>) {
        assert!(val1 >= val2);
        assert!(val1 < Some(val2.unwrap() + BUFFER_TIME_SECONDS));
    }

    pub fn assert_gt_with_buffer(val1: Option<u64>, val2: Option<u64>) {
        assert!(val1 > val2);
        assert!(val1 < Some(val2.unwrap() + BUFFER_TIME_SECONDS));
    }

    #[tokio::test]
    async fn set_time_in_past() {
        // set time and assert if it's greater/equal than past_time, check if it's inside buffer
        // limit
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
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than past_time, check if it's inside
        // buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(empty_block["timestamp"].as_u64(), Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block, check if it's inside buffer
        // limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(mint_block["timestamp"].as_u64(), empty_block["timestamp"].as_u64());
    }

    #[tokio::test]
    async fn set_time_in_future() {
        // set time and assert if it's greater/equal than future_time, check if it's inside buffer
        // limit
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
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than future_time, check if it's
        // inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(empty_block["timestamp"].as_u64(), Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last empty block, check if it's inside
        // buffer limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(mint_block["timestamp"].as_u64(), Some(future_time));
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

        // increase time and assert if it's greater than now, check if it's inside buffer limit
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
        assert_ge_with_buffer(
            first_increase_time_block["timestamp"].as_u64(),
            Some(now + first_increase_time),
        );

        // second increase time, check if it's inside buffer limit
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
        assert_ge_with_buffer(
            second_increase_time_block["timestamp"].as_u64(),
            Some(now + first_increase_time + second_increase_time),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check again if block_timestamp is greater than last block, check if it's
        // inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(
            empty_block["timestamp"].as_u64(),
            second_increase_time_block["timestamp"].as_u64(),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block, check if it's inside buffer
        // limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(last_block["timestamp"].as_u64(), empty_block["timestamp"].as_u64());
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

    #[tokio::test]
    async fn wrong_start_time() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--start-time", "wrong"]).await;
        assert!(devnet.is_err());
    }

    #[tokio::test]
    async fn start_time_in_past() {
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            past_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");

        // create block and check if block timestamp is greater/equal 1, check if it's inside buffer
        // limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(empty_block["timestamp"].as_u64(), Some(past_time));
    }

    #[tokio::test]
    async fn start_time_in_future() {
        let now = get_unix_timestamp_as_seconds();
        let future_time = now + 100;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            future_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");

        // create block and check if block timestamp is greater than now, check if it's inside
        // buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(empty_block["timestamp"].as_u64(), Some(future_time));
    }

    #[tokio::test]
    async fn advance_time_combination_test() {
        let now = get_unix_timestamp_as_seconds();
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            past_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");

        // increase time and assert if it's greater/equal than start-time argument +
        // first_increase_time, check if it's inside buffer limit
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
        assert_ge_with_buffer(
            first_increase_time_block["timestamp"].as_u64(),
            Some(past_time + first_increase_time),
        );

        // increase the time a second time and assert if it's greater/equal than past_time +
        // first_increase_time + second_increase_time, check if it's inside buffer limit
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
        assert_ge_with_buffer(
            second_increase_time_block["timestamp"].as_u64(),
            Some(past_time + first_increase_time + second_increase_time),
        );

        // set time to be now and check if the latest block timestamp is greater/equal now, check if
        // it's inside buffer limit
        let set_time_body = Body::from(
            json!({
                "time": now
            })
            .to_string(),
        );
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], now);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(now));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create a new empty block and check again if block timestamp is greater than
        // set_time_block, check if it's inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(
            empty_block["timestamp"].as_u64(),
            set_time_block["timestamp"].as_u64(),
        );

        // increase the time a third time and assert if it's greater/equal than last empty block
        // timestamp + third_increase_time, check if it's inside buffer limit
        let third_increase_time: u64 = 10000;
        let third_increase_time_body = Body::from(
            json!({
                "time": third_increase_time
            })
            .to_string(),
        );
        devnet.post_json("/increase_time".into(), third_increase_time_body).await.unwrap();
        let third_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            third_increase_time_block["timestamp"].as_u64(),
            Some(empty_block["timestamp"].as_u64().unwrap() + third_increase_time),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if the last block timestamp is greater previous block, check if it's inside buffer
        // limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            last_block["timestamp"].as_u64(),
            third_increase_time_block["timestamp"].as_u64(),
        );
    }
}
