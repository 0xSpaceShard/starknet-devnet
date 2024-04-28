use crate::common::background_devnet::BackgroundDevnet;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use peak_alloc::PeakAlloc;
use tokio::runtime::Runtime;

#[path = "../common/mod.rs"]
pub mod common;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

// peak memory in MB
const PEAK_MEMORY_LIMIT: f32 =  1.0;

static DUMMY_ADDRESS: u128 = 1;
static DUMMY_AMOUNT: u128 = 1;

async fn mint_iter(f: &str) {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", f])
        .await
        .expect("Could not start Devnet");

    for _n in 1..=5000 {
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
    }

    let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
    assert!(peak_mem < PEAK_MEMORY_LIMIT, "peak memory should not exceed {} in MB", PEAK_MEMORY_LIMIT);
    println!("The max amount that was used {} in MB of RAM", peak_mem);
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
