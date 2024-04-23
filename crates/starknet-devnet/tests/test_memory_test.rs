#![feature(test)]
#![feature(async_closure)]

extern crate test;
pub mod common;

#[cfg(test)]
mod memory_tests {
    use test::Bencher;

    use super::*;
    use crate::common::background_devnet::BackgroundDevnet;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    #[bench]
    fn bench_memory_with_mint(b: &mut Bencher) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        b.iter(|| {
            rt.block_on(async {
                let devnet = BackgroundDevnet::spawn_with_additional_args(&[
                    "--state-archive-capacity=full",
                ])
                .await
                .expect("Could not start Devnet");
                for n in 1..100000 {
                    println!("n: {:?}", n);
                    let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
                    println!("mint_tx_hash: {:?}", mint_tx_hash);
                }
            })
        });
    }
}
