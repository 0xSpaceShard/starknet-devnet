pub mod common;

mod advancing_time_tests {

    use hyper::Body;
    use serde_json::json;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn set_time_in_past() {
        // set time and assert
        let time = 1;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "time": time
            })
            .to_string(),
        );
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], time);

        // create block and check if block_timestamp is greater than time
        let resp_create_block = devnet
            .post_json("/create_block".into(), Body::from(json!({}).to_string()))
            .await
            .unwrap();
        let resp_body_create_block = get_json_body(resp_create_block).await;
        assert!(resp_body_create_block["block_timestamp"].as_u64() > Some(time))
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
        assert!(resp_body_create_block["block_timestamp"].as_u64() < Some(time))
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

    // TODO: same with increase time
}
