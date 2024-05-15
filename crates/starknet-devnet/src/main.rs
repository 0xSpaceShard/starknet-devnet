use anyhow::Ok;
use clap::Parser;
use cli::Args;
use server::api::json_rpc::RPC_SPEC_VERSION;
use server::api::Api;
use server::server::serve_http_api_json_rpc;
use starknet_core::account::Account;
use starknet_core::constants::{
    CAIRO_1_ERC20_CONTRACT_CLASS_HASH, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use starknet_core::starknet::starknet_config::{DumpOn, ForkConfig};
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{BlockId, BlockTag, MaybePendingBlockWithTxHashes};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_types::chain_id::ChainId;
use starknet_types::rpc::state::Balance;
use starknet_types::traits::ToHexString;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod cli;
mod initial_balance_wrapper;
mod ip_addr_wrapper;

/// Configures tracing with default level INFO,
/// If the environment variable `RUST_LOG` is set, it will be used instead.
fn configure_tracing() {
    let level_filter_layer =
        EnvFilter::builder().with_default_directive(tracing::Level::INFO.into()).from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(level_filter_layer).init();
}

fn log_predeployed_accounts(
    predeployed_accounts: &Vec<Account>,
    seed: u32,
    initial_balance: Balance,
) {
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
        println!("Initial balance of each account: {} WEI and FRI", initial_balance);
        println!("Seed to replicate this account sequence: {seed}");
    }
}

fn log_predeployed_contracts() {
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

fn log_chain_id(chain_id: ChainId) {
    println!("Chain ID: {} ({})", chain_id, chain_id.to_felt().to_prefixed_hex_str());
}

async fn check_forking_spec_version(
    client: &JsonRpcClient<HttpTransport>,
) -> Result<(), anyhow::Error> {
    let origin_spec_version = client.spec_version().await?;
    if origin_spec_version != RPC_SPEC_VERSION {
        warn!(
            "JSON-RPC API version of origin ({}) does not match this Devnet's version ({}).",
            origin_spec_version, RPC_SPEC_VERSION
        );
    }
    Ok(())
}

async fn check_forking_chain_id(
    client: &JsonRpcClient<HttpTransport>,
    devnet_chain_id: ChainId,
) -> Result<(), anyhow::Error> {
    let origin_chain_id = client.chain_id().await?;
    let devnet_chain_id_felt = devnet_chain_id.into();
    if origin_chain_id != devnet_chain_id_felt {
        warn!(
            "Origin chain ID ({:#x}) does not match this Devnet's chain ID ({:#x}).",
            origin_chain_id, devnet_chain_id_felt
        );
    }
    Ok(())
}

/// Logs forking info if forking specified. If block_number is not specified, it is set to the
/// latest block number.
pub async fn set_and_log_fork_config(
    fork_config: &mut ForkConfig,
    chain_id: ChainId,
) -> Result<(), anyhow::Error> {
    if let Some(url) = &fork_config.url {
        let json_rpc_client = JsonRpcClient::new(HttpTransport::new(url.clone()));
        let block_id =
            fork_config.block_number.map_or(BlockId::Tag(BlockTag::Latest), BlockId::Number);

        let block = json_rpc_client.get_block_with_tx_hashes(block_id).await.map_err(|e| {
            anyhow::Error::msg(match e {
                starknet_rs_providers::ProviderError::StarknetError(
                    starknet_rs_core::types::StarknetError::BlockNotFound,
                ) => format!("Forking from block {block_id:?}: block not found"),
                _ => format!("Forking from block {block_id:?}: {e}; Check the URL"),
            })
        })?;

        match block {
            MaybePendingBlockWithTxHashes::Block(b) => {
                fork_config.block_number = Some(b.block_number);
                println!("Forking from block: number={}, hash={:#x}", b.block_number, b.block_hash);
            }
            _ => panic!("Unreachable"),
        };

        check_forking_spec_version(&json_rpc_client).await?;
        check_forking_chain_id(&json_rpc_client, chain_id).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let (mut starknet_config, server_config) = args.to_config()?;

    set_and_log_fork_config(&mut starknet_config.fork_config, starknet_config.chain_id).await?;

    let address = format!("{}:{}", server_config.host, server_config.port);
    let listener = TcpListener::bind(address.clone()).await?;

    let api = Api::new(Starknet::new(&starknet_config)?);

    // set block timestamp shift during startup if start time is set
    if let Some(start_time) = starknet_config.start_time {
        api.starknet.write().await.set_block_timestamp_shift(
            start_time as i64 - Starknet::get_unix_timestamp_as_seconds() as i64,
        );
    };

    log_predeployed_contracts();
    log_chain_id(starknet_config.chain_id);

    let predeployed_accounts = api.starknet.read().await.get_predeployed_accounts();
    log_predeployed_accounts(
        &predeployed_accounts,
        starknet_config.seed,
        starknet_config.predeployed_accounts_initial_balance.clone(),
    );

    let server = serve_http_api_json_rpc(listener, api.clone(), &starknet_config, &server_config);

    info!("Starknet Devnet listening on {}", address);

    if starknet_config.dump_on == Some(DumpOn::Exit) {
        server.with_graceful_shutdown(shutdown_signal(api.clone())).await?
    } else {
        server.await?
    }

    Ok(())
}

pub async fn shutdown_signal(api: Api) {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");

    let starknet = api.starknet.read().await;
    starknet.dump_events().expect("Failed to dump starknet transactions");
}
