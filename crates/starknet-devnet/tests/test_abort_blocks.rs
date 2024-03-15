pub mod common;

mod abort_blocks_tests {
    use hyper::Body;
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn abort_latest_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();

        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        println!("last_block: {:?}", last_block["block_hash"]);

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "startingBlockHash": last_block["block_hash"] }).to_string()),
            )
            .await
            .unwrap();
        let abort_blocks_body = get_json_body(abort_blocks).await;
        println!("abort_blocks_body: {:?}", abort_blocks_body);

        let last_block_after_abort = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        println!("last_block_after_abort: {:?}", last_block_after_abort);
        println!("last_block_after_abort[status]: {:?}", last_block_after_abort["status"]);

        assert_eq!(last_block_after_abort["status"], "REJECTED".to_string());
    }
}
