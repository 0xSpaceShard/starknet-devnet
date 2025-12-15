#![allow(clippy::expect_used)]

use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramVec, IntCounter, IntCounterVec, Opts};

lazy_static! {
    /// Counter tracking total number of transactions in Starknet
    pub static ref TRANSACTION_COUNT: IntCounter = IntCounter::new(
        "starknet_transaction_count",
        "Total number of transactions in Starknet"
    )
    .expect("Failed to create TRANSACTION_COUNT counter");

    /// Counter tracking total number of blocks in Starknet
    pub static ref BLOCK_COUNT: IntCounter = IntCounter::new(
        "starknet_block_count",
        "Total number of blocks in Starknet"
    )
    .expect("Failed to create BLOCK_COUNT counter");

    /// Histogram tracking block creation duration in seconds
    pub static ref BLOCK_CREATION_DURATION: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "starknet_block_creation_duration_seconds",
            "Duration of block creation in seconds"
        )
        .buckets(vec![
            0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01
        ])
    )
    .expect("Failed to create BLOCK_CREATION_DURATION histogram");

    /// Histogram tracking upstream forking origin call duration in seconds
    pub static ref UPSTREAM_CALL_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "starknet_upstream_call_duration_seconds",
            "Duration of upstream forking origin calls in seconds"
        )
        .buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
        ]),
        &["method", "status"]
    )
    .expect("Failed to create UPSTREAM_CALL_DURATION histogram");

    /// Counter tracking total upstream forking origin calls
    pub static ref UPSTREAM_CALL_COUNT: IntCounterVec = IntCounterVec::new(
        Opts::new("starknet_upstream_call_count", "Total number of upstream forking origin calls"),
        &["method", "status"]
    )
    .expect("Failed to create UPSTREAM_CALL_COUNT counter");
}

/// Register all core metrics with the provided registry
pub fn register_metrics(registry: &prometheus::Registry) -> Result<(), prometheus::Error> {
    registry.register(Box::new(TRANSACTION_COUNT.clone()))?;
    registry.register(Box::new(BLOCK_COUNT.clone()))?;
    registry.register(Box::new(BLOCK_CREATION_DURATION.clone()))?;
    registry.register(Box::new(UPSTREAM_CALL_DURATION.clone()))?;
    registry.register(Box::new(UPSTREAM_CALL_COUNT.clone()))?;
    Ok(())
}
