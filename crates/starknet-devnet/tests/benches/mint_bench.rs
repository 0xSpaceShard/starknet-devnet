use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use tokio::runtime::Runtime;

use crate::common::background_devnet::BackgroundDevnet;

#[path = "../common/mod.rs"]
pub mod common;

static DUMMY_ADDRESS: u128 = 1;
static DUMMY_AMOUNT: u128 = 1;

async fn mint_iter(f: &str) {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&[
            format!("--state-archive-capacity={}", f).as_str()
        ])
        .await
        .expect("Could not start Devnet");

    for _n in 1..5000 {
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    }
}

fn bench_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("Mint");
    group.significance_level(0.1).sample_size(10);
    group.bench_function("full", |b| b.to_async(&rt).iter(|| black_box(mint_iter("full"))));
    group.bench_function("none", |b| b.to_async(&rt).iter(|| black_box(mint_iter("none"))));

    group.finish();
}

criterion_group!(benches, bench_memory);
criterion_main!(benches);
