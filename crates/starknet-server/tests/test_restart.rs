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

    #[tokio::test]
    async fn assert_tx_not_present_after_restart() {
        todo!();
    }

    #[tokio::test]
    async fn assert_storage_restarted() {
        todo!();
    }

    #[tokio::test]
    async fn assert_gas_price_unaffected_by_restart() {
        todo!();
    }

    #[tokio::test]
    async fn assert_predeployed_account_still_prefunded() {
        todo!();
    }
}
