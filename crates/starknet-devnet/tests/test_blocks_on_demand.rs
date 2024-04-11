pub mod common;

mod blocks_on_demand_tests {

    use crate::common::background_devnet::BackgroundDevnet;
    
    #[tokio::test]
    async fn blocks_on_demand() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        // TODO: add blocks-on-demand endpoint
    }

    #[tokio::test]
    async fn pending_block() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");
        let genesis_block_hash = devnet.get_latest_block_with_tx_hashes().await.unwrap().block_hash;
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();

        println!("genesis_block_hash: {:?}", genesis_block_hash);
        println!("pending_block: {:?}", pending_block);

        // devnet.create_block();
    }
}
