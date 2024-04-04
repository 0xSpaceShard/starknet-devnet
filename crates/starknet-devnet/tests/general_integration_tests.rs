// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_integration_tests {
    use hyper::{Body, StatusCode};
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    /// Asserts that a background instance can be spawned
    async fn spawnable() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn too_big_request_rejected() {
        let limit = 1_000;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--request-body-size-limit",
            &limit.to_string(),
        ])
        .await
        .unwrap();

        let too_big_path = "a".repeat(limit);
        match devnet
            .post_json("/load".into(), Body::from(json!({ "path": too_big_path }).to_string()))
            .await
        {
            Ok(resp) => {
                assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
            }
            other => panic!("Unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn request_size_below_limit() {
        let limit = 100;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--request-body-size-limit",
            &limit.to_string(),
        ])
        .await
        .unwrap();

        // subtract enough so that the rest of the json body doesn't overflow the limit
        let ok_path = "0".repeat(limit - 20);
        match devnet
            .post_json("/load".into(), Body::from(json!({ "path": ok_path }).to_string()))
            .await
        {
            Ok(resp) => {
                assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
                let load_resp = get_json_body(resp).await;
                assert_eq!(load_resp, json!({ "error": "The file does not exist" }));
            }
            other => panic!("Unexpected response: {other:?}"),
        }
    }
}
