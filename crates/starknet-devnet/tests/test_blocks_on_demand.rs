pub mod common;

mod blocks_on_demand_tests {

    use crate::common::background_devnet::BackgroundDevnet;
    
    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn blocks_on_demand() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");
        
        let x1 = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        println!("x1: {:?}", x1);

        let x2 = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        println!("x2: {:?}", x2);
        
        let x3 = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        println!("x3: {:?}", x3);

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        println!("latest_block: {:?}", latest_block);

        let new_block_hash = devnet.create_block().await.unwrap();
        println!("new_block_hash: {:?}", new_block_hash);

        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        println!("latest_block: {:?}", latest_block);
        println!("latest_block.block_number: {:?}", latest_block.block_number);
        println!("latest_block.transactions.len(): {:?}", latest_block.transactions.len());
    }
}
