#![cfg(test)]
pub mod common;

mod reorg_subscription_support {
    use std::collections::HashMap;

    use serde_json::json;
    use starknet_rs_core::types::BlockId;
    use tokio_tungstenite::connect_async;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        assert_no_notifications, receive_rpc_via_ws, subscribe, unsubscribe,
    };

    #[tokio::test]
    async fn reorg_notification_for_all_subscriptions_except_pending_tx() {
        let devnet_args = ["--state-archive-capacity", "full"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        // create blocks for later abortion
        let starting_block_hash = devnet.create_block().await.unwrap();
        let ending_block_hash = devnet.create_block().await.unwrap();

        let mut notifiable_subscribers = HashMap::new();
        for (subscription_method, subscription_params) in [
            ("starknet_subscribeNewHeads", json!({})),
            ("starknet_subscribeTransactionStatus", json!({ "transaction_hash": "0x1" })),
            // TODO ("starknet_subscribeEvents", json!({})),
        ] {
            let (mut ws, _) = connect_async(devnet.ws_url()).await.unwrap();
            let subscription_id =
                subscribe(&mut ws, subscription_method, subscription_params).await.unwrap();
            notifiable_subscribers.insert(subscription_id, ws);
        }

        let (mut unnotifiable_ws, _) = connect_async(devnet.ws_url()).await.unwrap();
        subscribe(&mut unnotifiable_ws, "starknet_subscribePendingTransactions", json!({}))
            .await
            .unwrap();

        // assert that block-, tx_status- and event-subscribers got notified; unsubscribe them
        devnet.abort_blocks(&BlockId::Hash(ending_block_hash)).await.unwrap();
        for (subscription_id, ws) in notifiable_subscribers.iter_mut() {
            let notification = receive_rpc_via_ws(ws).await.unwrap();
            assert_eq!(
                notification,
                json!({
                    "jsonrpc": "2.0",
                    "method": "starknet_subscriptionReorg",
                    "params": {
                        "result": {
                            "starting_block_hash": starting_block_hash,
                            "starting_block_number": 1,
                            "ending_block_hash": ending_block_hash,
                            "ending_block_number": 2,
                        },
                        "subscription_id": subscription_id,
                    }
                })
            );
            unsubscribe(ws, *subscription_id).await.unwrap();
        }

        // now that all sockets are unsubscribed, abort a new block and assert no notifications
        let additional_block_hash = devnet.create_block().await.unwrap();
        devnet.abort_blocks(&BlockId::Hash(additional_block_hash)).await.unwrap();
        for (_, mut ws) in notifiable_subscribers {
            assert_no_notifications(&mut ws).await;
        }

        assert_no_notifications(&mut unnotifiable_ws).await;
    }

    #[tokio::test]
    async fn socket_with_two_subscriptions_should_get_one_reorg_notification() {
        unimplemented!();
    }
}
