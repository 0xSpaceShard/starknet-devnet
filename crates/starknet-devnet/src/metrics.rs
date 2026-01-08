use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use lazy_static::lazy_static;
use prometheus::{Encoder, Registry, TextEncoder};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

lazy_static! {
    pub static ref METRICS_REGISTRY: Arc<Registry> = {
        let registry = Arc::new(Registry::new());

        // Register all metrics from each crate using their bulk registration functions
        if let Err(e) = server::metrics::register_metrics(&registry) {
            warn!("Failed to register server metrics: {}", e);
        }
        if let Err(e) = starknet_core::metrics::register_metrics(&registry) {
            warn!("Failed to register core metrics: {}", e);
        }

        registry
    };
}

/// Get metrics in Prometheus text format
pub fn get_metrics() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;
    String::from_utf8(buffer)
        .map_err(|e| prometheus::Error::Msg(format!("Failed to convert metrics to UTF-8: {}", e)))
}

/// Axum handler for metrics endpoint
async fn metrics_handler() -> Response {
    match get_metrics() {
        Ok(metrics) => (StatusCode::OK, [("content-type", "text/plain; version=0.0.4")], metrics)
            .into_response(),
        Err(e) => {
            error!("Error gathering metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("Error gathering metrics: {}", e),
            )
                .into_response()
        }
    }
}

/// Start the metrics server on the specified address
pub async fn start_metrics_server(addr: SocketAddr) -> Result<(), std::io::Error> {
    let app = Router::new().route("/metrics", get(metrics_handler));

    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    info!("Listening at http://{}/metrics", local_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}
