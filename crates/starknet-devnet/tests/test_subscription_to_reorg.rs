#![cfg(test)]
pub mod common;

mod reorg_subscription_support {
    use tokio_tungstenite::connect_async;

    use crate::common::background_devnet::BackgroundDevnet;

    #[tokio::test]
    async fn reorg_notification_only_for_some_subscriptions() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (mut _ws, _) = connect_async(devnet.ws_url()).await.unwrap();
    }

    #[tokio::test]
    async fn should_not_notify_after_unsubscription() {
        unimplemented!();
    }

    #[tokio::test]
    async fn socket_with_two_subscriptions_should_get_one_reorg_notification() {
        unimplemented!();
    }

    #[tokio::test]
    async fn restarting_should_forget_all_subscribers_and_not_notify_of_reorg() {
        unimplemented!()
    }
}
