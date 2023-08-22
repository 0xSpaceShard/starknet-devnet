use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

use ::server::ServerConfig;
use clap::Parser;
use cli::Args;
use starknet_core::account::Account;
use starknet_core::starknet::Starknet;
use starknet_types::felt::Felt;
use starknet_types::traits::{ToDecimalString, ToHexString};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tokio_graceful_shutdown::Toplevel;

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

async fn main2(subsys: SubsystemHandle) -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let starknet_config = args.to_starknet_config();
    let host =
        IpAddr::from_str(starknet_config.host.as_str()).expect("Invalid value for host IP address");
    let mut addr = SocketAddr::new(host, starknet_config.port);

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

async fn main1(subsys: SubsystemHandle) -> Result<(), anyhow::Error> {
    subsys.start("Main2", main2);
    subsys.on_shutdown_requested().await;
    println!("Subsystem2 stopped.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    Toplevel::new()
        .start("Main1", main1)
        .catch_signals()
        .handle_shutdown_requests(Duration::from_millis(1000))
        .await
        .map_err(Into::into)
}