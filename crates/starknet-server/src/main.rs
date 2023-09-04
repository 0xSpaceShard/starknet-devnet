use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use api::Api;
use ::server::ServerConfig;
use clap::Parser;
use cli::Args;
use starknet_core::account::Account;
use starknet_core::starknet::{Starknet, DumpMode};
use starknet_core::transactions::StarknetTransactions;
use starknet_types::felt::Felt;
use starknet_types::rpc::transactions::Transaction;
use starknet_types::traits::{ToDecimalString, ToHexString};
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod api;
mod cli;
mod server;

use std::fs;
use std::path::Path;
use std::io;
use std::io::prelude::*;
use std::fs::File;

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

    let api = api::Api::new(Starknet::new(&starknet_config)?);

    // match &starknet_config.dump_path {
    //     Some(path) => {
    //         println!("load: {:?}", encoded); // remove this println
    
    //         // let mut f = File::open(&Path::new(path)).unwrap();
    //         // let mut v: Vec<u8> = Vec::new();
    //         // let file_content = f.read_to_end(&mut v);
    //         // println!("file_content: {:?}", file_content);
    //         // println!("v: {:?}", v);
    //         // let decoded: Option<String> = bincode::deserialize(&v[..]).unwrap();
    //         // println!("assert: {:?}", assert_eq!(data.clone(), decoded.clone()));
    //         // println!("decoded: {:?}", decoded);
    //         // let txs: StarknetTransactions = serde_json::from_str(decoded.unwrap().as_str()).unwrap();
    //         // println!("txs: {:?}", txs);

    //         let data = Some(serde_json::to_string(&starknet.transactions).unwrap_or_default());
    //         let encoded: Vec<u8> = bincode::serialize(&data).unwrap_or_default();
    //         fs::write(Path::new(path), encoded).unwrap_or_default();
    //     },
    //     _ => println!("Dump path is not set."),
    // }


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

fn is_dump_on(dump_on: &Option<DumpMode> ) -> bool {
    match dump_on {
        None => false,
        Some(dump_on) => {
            *dump_on == DumpMode::OnExit
        },
    }
}

pub async fn shutdown_signal(api: Api) -> (){
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    // Wait for the CTRL+C signal
    signal::ctrl_c().await;

    // Dump StarknetTransactions
    let starknet: tokio::sync::RwLockReadGuard<'_, Starknet> = api.starknet.read().await;
    if is_dump_on(&starknet.config.dump_on) {
        match &starknet.config.dump_path {
            Some(path) => {
                let data = Some(serde_json::to_string(&starknet.transactions).unwrap_or_default());
                let encoded: Vec<u8> = bincode::serialize(&data).unwrap_or_default();
                fs::write(Path::new(path), encoded).unwrap_or_default();
            },
            _ => info!("Failed to dump transactions, dump path is not set."),
        }
    }
}
