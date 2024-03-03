use std::net::SocketAddr;

use anyhow::Ok;
use clap::Parser;
use cli::Args;
use server::api::Api;
use server::server::serve_http_api_json_rpc;
use server::ServerConfig;
use starknet_core::account::Account;
use starknet_core::constants::{
    CAIRO_1_ERC20_CONTRACT_CLASS_HASH, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use starknet_core::starknet::starknet_config::DumpOn;
use starknet_core::starknet::Starknet;
use starknet_types::chain_id::ChainId;
use starknet_types::felt::Felt;
use starknet_types::traits::{ToDecimalString, ToHexString};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod cli;
mod contract_class_choice;
mod initial_balance_wrapper;
mod ip_addr_wrapper;

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
        let class_hash = predeployed_accounts.get(0).unwrap().class_hash.to_prefixed_hex_str();
        println!("Predeployed accounts using class with hash: {class_hash}");
        println!(
            "Initial balance of each account: {} WEI and FRI",
            initial_balance.to_decimal_string()
        );
        println!("Seed to replicate this account sequence: {seed}");
    }
}

fn print_predeployed_contracts() {
    println!("Predeployed FeeToken");
    println!("ETH Address: {ETH_ERC20_CONTRACT_ADDRESS}");
    println!("STRK Address: {STRK_ERC20_CONTRACT_ADDRESS}");
    println!("Class Hash: {CAIRO_1_ERC20_CONTRACT_CLASS_HASH}");
    println!();
    println!("Predeployed UDC");
    println!("Address: {UDC_CONTRACT_ADDRESS}");
    println!("Class Hash: {UDC_CONTRACT_CLASS_HASH}");
    println!();
}

fn print_chain_id(chain_id: ChainId) {
    println!("Chain ID: {} ({})", chain_id, chain_id.to_felt().to_prefixed_hex_str());
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let starknet_config = args.to_starknet_config()?;
    let mut addr: SocketAddr = SocketAddr::new(starknet_config.host, starknet_config.port);

    let api = Api::new(Starknet::new(&starknet_config)?);

    // set block timestamp shift during startup if start time is set
    if let Some(start_time) = starknet_config.start_time {
        api.starknet.write().await.set_block_timestamp_shift(
            start_time as i64 - Starknet::get_unix_timestamp_as_seconds() as i64,
        );
    };

    print_predeployed_contracts();
    print_chain_id(starknet_config.chain_id);

    let predeployed_accounts = api.starknet.read().await.get_predeployed_accounts();
    log_predeployed_accounts(
        &predeployed_accounts,
        starknet_config.seed,
        starknet_config.predeployed_accounts_initial_balance,
    );

    let server =
        serve_http_api_json_rpc(addr, ServerConfig::default(), api.clone(), &starknet_config);
    addr = server.local_addr();

    info!("Starknet Devnet listening on {}", addr);

    // spawn the server on a new task
    let serve = if starknet_config.dump_on == Some(DumpOn::Exit) {
        tokio::task::spawn(server.with_graceful_shutdown(shutdown_signal(api.clone())))
    } else {
        tokio::task::spawn(server)
    };

    Ok(serve.await??)
}

pub async fn shutdown_signal(api: Api) {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");

    let starknet = api.starknet.read().await;
    starknet.dump_events().expect("Failed to dump starknet transactions");
}
