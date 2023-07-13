mod common;

#[cfg(test)]
mod integration_tests {
    use crate::common::BackgroundDevnet;

    #[tokio::test]
    async fn spawnable() {
        let mut devnet = BackgroundDevnet::new();
        devnet.spawn().await.expect("Could not start Devnet");
    }
}
