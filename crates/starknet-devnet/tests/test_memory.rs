pub mod common;

mod memory_test {

    use peak_alloc::PeakAlloc;

    use crate::common::background_devnet::BackgroundDevnet;

    #[global_allocator]
    static PEAK_ALLOC: PeakAlloc = PeakAlloc;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn mint_iter(capacity: &str) -> f32 {
        // reset peak usage
        PEAK_ALLOC.reset_peak_usage();

        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", capacity])
                .await
                .expect("Could not start Devnet");

        for _ in 1..=5000 {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        // set and return peak usage after recurring mint run
        PEAK_ALLOC.peak_usage_as_mb()
    }

    #[tokio::test]
    async fn test_full_state_archive_capacity_presents_no_increase() {
        // tolerance limit of 0.1%
        const TOLERANCE: f32 = 0.001;
        let full = mint_iter("full").await;
        let none = mint_iter("none").await;

        let diff = (full - none).abs() / none;
        assert!(
            diff <= TOLERANCE,
            "Memory difference should not exceed {}% tolerance.
    Current difference is {}% between:
    - Full state archive: {full} MB
    - No state archive: {none} MB",
            TOLERANCE * 100.0,
            diff * 100.0,
        );
    }
}
