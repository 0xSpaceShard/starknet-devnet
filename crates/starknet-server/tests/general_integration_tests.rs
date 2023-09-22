// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_integration_tests {
    use hyper::{Body, Response};
    use serde_json::json;

    use crate::common::constants::RPC_PATH;
    use crate::common::devnet::BackgroundDevnet;

    #[tokio::test]
    /// Asserts that a background instance can be spawned
    async fn spawnable() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn rpc_at_root() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        async fn extract_body_as_string(resp: Response<Body>) -> String {
            let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap().to_vec();
            String::from_utf8(bytes).unwrap()
        }

        let resp_root =
            devnet.post_json("/".into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_root_body = extract_body_as_string(resp_root).await;

        let resp_rpc =
            devnet.post_json(RPC_PATH.into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_rpc_body = extract_body_as_string(resp_rpc).await;

        assert_eq!(resp_root_body, resp_rpc_body);
    }
}
