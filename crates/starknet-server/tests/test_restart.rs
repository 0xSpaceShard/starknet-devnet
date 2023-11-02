// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_restart {
    use hyper::{Body, StatusCode};

    use crate::common::devnet::BackgroundDevnet;

    #[tokio::test]
    async fn assert_restartable() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.post_json("/restart".into(), Body::empty()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // TODO add more
}
