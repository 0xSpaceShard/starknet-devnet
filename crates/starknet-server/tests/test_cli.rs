// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod cli {
    use crate::common::devnet::BackgroundDevnet;

    #[tokio::test]
    async fn test_invalid_host_rejected() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }

    #[tokio::test]
    async fn localhost_considered_valid() {
        BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    }
}
