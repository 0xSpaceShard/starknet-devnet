pub mod common;

mod advancing_time_tests {

    use std::time::SystemTime;
    use std::{thread, time};

    use hyper::Body;
    use serde_json::json;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn set_time_in_past() {
        // set time and assert
        let time_to_set = 1;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "time": time_to_set
            })
            .to_string(),
        );
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], time_to_set);

        // create block and check if block_timestamp is greater than time_to_set
        let resp_create_block = devnet
            .post_json("/create_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_create_block = get_json_body(resp_create_block).await;
        assert!(resp_body_create_block["block_timestamp"].as_u64() > Some(time_to_set));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint block_timestamp is greater than create block timestamp
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let resp_latest_block = devnet
            .post_json("/get_latest_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_latest_block = get_json_body(resp_latest_block).await;
        assert!(
            resp_body_latest_block["block_timestamp"].as_u64()
                > resp_body_create_block["block_timestamp"].as_u64()
        );
    }

    #[tokio::test]
    async fn set_time_in_far_future() {
        // set time and assert
        let time: u64 = 3376684800;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "time": time
            })
            .to_string(),
        );
        let resp = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body = get_json_body(resp).await;
        assert_eq!(resp_body["block_timestamp"], time);

        // create block and check if block_timestamp is less than time
        let resp_create_block = devnet
            .post_json("/create_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_create_block = get_json_body(resp_create_block).await;
        assert!(resp_body_create_block["block_timestamp"].as_u64() < Some(time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint block_timestamp is greater than create block timestamp
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let resp_latest_block = devnet
            .post_json("/get_latest_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_latest_block = get_json_body(resp_latest_block).await;
        assert!(
            resp_body_latest_block["block_timestamp"].as_u64()
                > resp_body_create_block["block_timestamp"].as_u64()
        );
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
        // increase time and assert if it's greater than now
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("should get current UNIX timestamp")
            .as_secs();
        let time: u64 = 3600;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let increase_time_body = Body::from(
            json!({
                "time": time
            })
            .to_string(),
        );
        let resp_increase_time =
            devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();
        let resp_body_increase_time = get_json_body(resp_increase_time).await;
        assert!(resp_body_increase_time["block_timestamp"].as_u64() > Some(now));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check again if block_timestamp is greater than than last block
        let resp_create_block = devnet
            .post_json("/create_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_create_block = get_json_body(resp_create_block).await;
        assert!(
            resp_body_create_block["block_timestamp"].as_u64()
                > resp_body_increase_time["block_timestamp"].as_u64()
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint block_timestamp is greater than create block timestamp
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let resp_latest_block = devnet
            .post_json("/get_latest_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_latest_block = get_json_body(resp_latest_block).await;
        assert!(
            resp_body_latest_block["block_timestamp"].as_u64()
                > resp_body_create_block["block_timestamp"].as_u64()
        );
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
