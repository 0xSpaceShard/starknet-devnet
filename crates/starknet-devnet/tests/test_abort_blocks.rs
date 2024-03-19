pub mod common;

mod abort_blocks_tests {
    use hyper::Body;
    use serde_json::json;
    use starknet_rs_core::types::{FieldElement, TransactionStatus};
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn abort_latest_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "startingBlockHash": second_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);

        let first_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": first_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(first_block_after_abort["status"], "ACCEPTED_ON_L2".to_string());

        let second_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": second_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(second_block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_two_blocks() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let first_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let second_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "startingBlockHash": first_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], second_block["block_hash"]);
        assert_eq!(aborted_blocks["aborted"][1], first_block["block_hash"]);

        let first_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": first_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(first_block_after_abort["status"], "REJECTED".to_string());

        let second_block_after_abort = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithTxHashes",
                json!({
                    "block_id": {"block_hash": second_block["block_hash"]},
                }),
            )
            .await["result"];
        assert_eq!(second_block_after_abort["status"], "REJECTED".to_string());
    }

    #[tokio::test]
    async fn abort_block_with_transaction() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let mint_hash = devnet.mint(FieldElement::ONE, 100).await;

        let latest_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "startingBlockHash": latest_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();

        let aborted_blocks = get_json_body(abort_blocks).await;
        assert_eq!(aborted_blocks["aborted"][0], latest_block["block_hash"]);

        let tx_status_after_abort =
            devnet.json_rpc_client.get_transaction_status(mint_hash).await.unwrap();

        assert_eq!(
            tx_status_after_abort,
            TransactionStatus::AcceptedOnL2(
                starknet_rs_core::types::TransactionExecutionStatus::Reverted
            )
        );
    }

    // TODO: add tests with block number queries
}
