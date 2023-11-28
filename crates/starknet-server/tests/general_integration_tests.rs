// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod general_integration_tests {
    use hyper::Body;
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::RPC_PATH;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    /// Asserts that a background instance can be spawned
    async fn spawnable() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn rpc_at_root() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let resp_root =
            devnet.post_json("/".into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_root_body = get_json_body(resp_root).await;

        let resp_rpc =
            devnet.post_json(RPC_PATH.into(), Body::from(json!({}).to_string())).await.unwrap();
        let resp_rpc_body = get_json_body(resp_rpc).await;

        assert_eq!(resp_root_body, resp_rpc_body);
    }
}
