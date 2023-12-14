pub mod common;

mod memory_tests {
    use crate::common::background_devnet::BackgroundDevnet;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn test_memory_with_mint() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        for n in 1..5000 {
            println!("n: {:?}", n);
            let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            println!("mint_tx_hash: {:?}", mint_tx_hash);
        }
    }
}
