pub mod common;

mod memory_test {

    use peak_alloc::PeakAlloc;

    use crate::common::background_devnet::BackgroundDevnet;

    #[global_allocator]
    static PEAK_ALLOC: PeakAlloc = PeakAlloc;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn mint_iter(f: &str) -> f32 {
        // reset peak usage
        PEAK_ALLOC.reset_peak_usage();

        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", f])
            .await
            .expect("Could not start Devnet");

        for n in 1..=5_000 {
            println!("n: {:?}", n);
            let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            println!("mint_tx_hash: {:?}", mint_tx_hash);
        }

        // set and return peak usage after recurring mint run
        PEAK_ALLOC.peak_usage_as_mb()
    }

    #[tokio::test]
    async fn test_memory_limit() {
        // tolerance limit of 0.1%
        const TOLERANCE: f32 = 0.001;
        let full = mint_iter("full").await;
        let none = mint_iter("none").await;

        let diff = (full - none) / none;
        println!("Diff: {}", diff);
        println!("Full mem in MB: {}", full);
        println!("Full mem in MB: {}", none);

        assert!(diff < TOLERANCE, "memory difference should not exceed more than {}% in tolerance. Current Difference {}% between full state archive in {}MB and no state archive in {}MB", TOLERANCE * 100.0, diff * 100.0, full, none);
    }
}
