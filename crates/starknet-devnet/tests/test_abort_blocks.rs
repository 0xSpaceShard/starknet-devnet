pub mod common;

mod abort_blocks_tests {
    use hyper::Body;
    use serde_json::json;

    use crate::common::background_devnet::BackgroundDevnet;

    #[tokio::test]
    async fn abort_latest_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();

        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        println!("last_block: {:?}", last_block);

        let abort_blocks = devnet
            .post_json(
                "/abort_blocks".into(),
                Body::from(json!({ "startingBlockHash": "0x0" }).to_string()),
            )
            .await
            .unwrap();
        println!("abort_blocks: {:?}", abort_blocks);

        let last_block_after_abort = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        println!("last_block_after_abort: {:?}", last_block_after_abort);
    }
}
