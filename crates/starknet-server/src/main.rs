use std::net::SocketAddr;

use ::server::ServerConfig;
use clap::Parser;
use cli::Args;
use starknet_core::account::Account;
use starknet_core::starknet::Starknet;
use starknet_types::felt::Felt;
use starknet_types::traits::{ToDecimalString, ToHexString};
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

fn log_predeployed_accounts(predeployed_accounts: &Vec<Account>, seed: u32, initial_balance: Felt) {
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

    if !predeployed_accounts.is_empty() {
        println!();
        println!("Initial balance of each account: {} WEI", initial_balance.to_decimal_string());
        println!("Seed to replicate this account sequence: {seed}");
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let starknet_config = args.to_starknet_config();
    let mut addr: SocketAddr = SocketAddr::new(starknet_config.host, starknet_config.port);

    let api = api::Api::new(Starknet::new(&starknet_config)?);

    let predeployed_accounts = api.starknet.read().await.get_predeployed_accounts();
    log_predeployed_accounts(
        &predeployed_accounts,
        starknet_config.seed,
        starknet_config.predeployed_accounts_initial_balance,
    );

    let server = server::serve_http_api_json_rpc(
        addr,
        ServerConfig::default(),
        api.clone(),
        &starknet_config,
    );
    addr = server.local_addr();

    info!("Starknet Devnet listening on {}", addr);

    // spawn the server on a new task
    let serve = tokio::task::spawn(server);

    Ok(serve.await??)
}
