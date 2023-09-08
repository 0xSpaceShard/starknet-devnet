use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use ::server::ServerConfig;
use api::Api;
use clap::Parser;
use cli::Args;
use starknet_core::account::Account;
use starknet_core::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_CLASS_HASH, UDC_CONTRACT_CLASS_HASH,
};
use starknet_core::starknet::{DumpMode, Starknet};
use starknet_core::transactions::StarknetTransactions;
use starknet_types::felt::{Felt, ClassHash};
use starknet_types::traits::{ToDecimalString, ToHexString};
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod api;
mod cli;
mod server;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

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
    let host =
        IpAddr::from_str(starknet_config.host.as_str()).expect("Invalid value for host IP address");
    let mut addr = SocketAddr::new(host, starknet_config.port);

    // Load Starknet transactions and contracts from file
    let mut transactions: StarknetTransactions = StarknetTransactions::default();
    let mut contracts = HashMap::new();
    if let Some(path) = &starknet_config.dump_path {
        let file_path = Path::new(path);
        if file_path.exists() {
            let mut file = File::open(file_path).expect("Failed to open file");
            let mut v: Vec<u8> = Vec::new();
            file.read_to_end(&mut v).expect("Failed to read from file");
            let decoded: Option<String> =
                bincode::deserialize(&v[..]).expect("Failed to deserialize state");
            let mut starknet: Starknet = Starknet::default();
            starknet = serde_json::from_str(decoded.unwrap().as_str())
                .expect("Failed to decode state");
            transactions = starknet.transactions;
            // println!("{:?}", transactions);
            contracts = starknet.state.contract_classes;
            println!("{:?}", contracts);
        }
    }

    let api = api::Api::new(Starknet::new(&starknet_config, Some(transactions), Some(contracts))?);

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
    let serve = tokio::task::spawn(server.with_graceful_shutdown(shutdown_signal(api.clone())));

    Ok(serve.await??)
}

fn is_dump_on(dump_on: &Option<DumpMode>) -> bool {
    match dump_on {
        None => false,
        Some(dump_on) => *dump_on == DumpMode::OnExit,
    }
}

pub async fn shutdown_signal(api: Api) {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");

    // Wait for the CTRL+C signal
    signal::ctrl_c().await.expect("Failed to read CTRL+C signal");

    // Save Starknet do file
    let starknet = api.starknet.read().await;
    if is_dump_on(&starknet.config.dump_on) {
        match &starknet.config.dump_path {
            Some(path) => {
                let starknet_dump = Some(
                    serde_json::to_string(starknet.get_starknet())
                        .expect("Failed to serialize starknet object"),
                );
                let encoded: Vec<u8> =
                    bincode::serialize(&starknet_dump).expect("Failed to encode starknet object");
                fs::write(Path::new(path), encoded).expect("Failed to save starknet object");
            }
            _ => info!("Failed to dump starknet object, dump path is not set"),
        }
    }
}
