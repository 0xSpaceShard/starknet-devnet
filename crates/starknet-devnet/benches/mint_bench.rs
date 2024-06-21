use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use crate::common::background_devnet::BackgroundDevnet;

#[path = "../tests/common/mod.rs"]
pub mod common;

static DUMMY_ADDRESS: u128 = 1;
static DUMMY_AMOUNT: u128 = 1;

async fn mint_iter(capacity: &str) {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", capacity])
            .await
            .expect("Could not start Devnet");

    for _ in 1..=2_500 {
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    }
}

fn bench_devnet(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("Mint");
    group.significance_level(0.1).sample_size(10);
    for i in ["full", "none"].iter() {
        group.bench_function(*i, |b| b.to_async(&rt).iter(|| black_box(mint_iter(i))));
    }

    group.finish();
}

criterion_group!(benches, bench_devnet);
criterion_main!(benches);
