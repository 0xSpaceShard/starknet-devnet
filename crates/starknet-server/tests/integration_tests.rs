mod common;

#[cfg(test)]
mod integration_tests {
    use crate::common::BackgroundDevnet;

    #[tokio::test]
    async fn spawnable() {
        BackgroundDevnet::spawn().await;
    }
}
