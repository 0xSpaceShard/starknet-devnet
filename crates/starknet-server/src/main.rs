use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use ::server::ServerConfig;
use clap::Parser;
use cli::Args;
use starknet_core::Starknet;
use starknet_types::traits::ToHexString;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod api;
mod cli;
mod server;

/// Configures tracing with default level INFO,
/// If the environment variable `RUST_LOG` is set, it will be used instead.
fn configure_tracing() {
    let level_filter_layer =
        EnvFilter::builder().with_default_directive(tracing::Level::INFO.into()).from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(level_filter_layer).init();
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let starknet_config = args.to_starknet_config();

    // configure server
    let port = env::var("DEVNET_PORT")
        .expect("DEVNET_PORT must be set")
        .parse::<u16>()
        .expect("DEVNET_PORT must be a valid port number");

    let host = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let mut addr = SocketAddr::new(host, port);

    let api = api::Api::new(Starknet::new(&starknet_config)?);

    let predeployed_accounts = api.starknet.read().await.get_predeployed_accounts();
    for account in predeployed_accounts {
        let formatted_str = format!(
            r"
| Account address |  {} 
| Private key     |  {}
| Public key      |  {}",
            account.account_address.to_prefixed_hex_str(),
            account.private_key.to_prefixed_hex_str(),
            account.public_key.to_prefixed_hex_str()
        );

        println!("{}", formatted_str);
    }

    let server = server::serve_http_api_json_rpc(addr, ServerConfig::default(), api.clone());
    addr = server.local_addr();

    info!("StarkNet Devnet listening on {}", addr);

    // spawn the server on a new task
    let serve = tokio::task::spawn(server);

    Ok(serve.await??)
}
