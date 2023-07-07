use std::{
    net::{IpAddr, SocketAddr}, str::FromStr,
};

use ::server::ServerConfig;
use tracing::info;
use tracing_subscriber::EnvFilter;
use starknet_core::StarknetConfig;

mod api;
mod server;

/// Configures tracing with default level INFO,
/// If the environment variable `RUST_LOG` is set, it will be used instead.
fn configure_tracing() {
    let level_filter_layer =
        EnvFilter::builder().with_default_directive(tracing::Level::INFO.into()).from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(level_filter_layer).init();
}

pub async fn start_server(starknet_config: &StarknetConfig) -> Result<(), anyhow::Error> {
    configure_tracing();
    let host = IpAddr::from_str(starknet_config.host.as_str()).unwrap();
    let mut addr = SocketAddr::new(host, starknet_config.port);
    let server = server::serve_http_api_json_rpc(addr, ServerConfig::default(), starknet_config);
    addr = server.local_addr();

    info!("StarkNet Devnet listening on {}", addr);

    // spawn the server on a new task
    let serve = tokio::task::spawn(server);

    Ok(serve.await??)
}
