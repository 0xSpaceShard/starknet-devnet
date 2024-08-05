use std::collections::HashSet;
use std::future::IntoFuture;
use std::result::Result::Ok;
use std::time::Duration;

use clap::Parser;
use cli::Args;
use futures::future::join_all;
use server::api::json_rpc::{JsonRpcRequest, RPC_SPEC_VERSION};
use server::api::Api;
use server::server::{serve_http_api_json_rpc, HTTP_API_ROUTES_WITHOUT_LEADING_SLASH};
use starknet_core::account::Account;
use starknet_core::constants::{
    CAIRO_1_ERC20_CONTRACT_CLASS_HASH, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use starknet_core::starknet::starknet_config::{BlockGenerationOn, DumpOn, ForkConfig};
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{BlockId, BlockTag, MaybePendingBlockWithTxHashes};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_types::chain_id::ChainId;
use starknet_types::rpc::state::Balance;
use starknet_types::traits::ToHexString;
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tokio::task::{self};
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod cli;
mod initial_balance_wrapper;
mod ip_addr_wrapper;

const REQUEST_LOG_ENV_VAR: &str = "request";
const RESPONSE_LOG_ENV_VAR: &str = "response";

/// Configures tracing with default level INFO,
/// If the environment variable `RUST_LOG` is set, it will be used instead.
/// Added are two more directives: `request` and `response`. If they are present, then have to be
/// removed to be able to construct the `EnvFilter` correctly, because tracing_subscriber recognizes
/// them as path syntax (way to access a module) and assigns them TRACE level. Because they are not
/// paths to some module like this one: `starknet-devnet::cli` nothing gets logged. For example:
/// `RUST_LOG=request` is translated to `request=TRACE`, which means that will log TRACE level for
/// request module.
fn configure_tracing() {
    let log_env_var = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default().to_lowercase();

    // Remove the `request` and `response` directives from the environment variable.
    // And trim empty spaces around each directive
    let log_env_var = log_env_var
        .split(',')
        .map(|el| el.trim())
        .filter(|el| ![REQUEST_LOG_ENV_VAR, RESPONSE_LOG_ENV_VAR].contains(el))
        .collect::<Vec<&str>>()
        .join(",");

    let level_filter_layer = EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .parse_lossy(log_env_var);

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
            account.private_key.to_fixed_hex_string(),
            account.public_key.to_fixed_hex_string()
        );

        println!("{}", formatted_str);
    }

    if !predeployed_accounts.is_empty() {
        println!();
        let class_hash = predeployed_accounts.first().unwrap().class_hash.to_fixed_hex_string();
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

fn log_chain_id(chain_id: &ChainId) {
    println!("Chain ID: {} ({})", chain_id, chain_id.to_felt().to_hex_string());
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

/// Logs forking info if forking specified. If block_number is not specified, it is set to the
/// latest block number.
pub async fn set_and_log_fork_config(
    fork_config: &mut ForkConfig,
    json_rpc_client: &JsonRpcClient<HttpTransport>,
) -> Result<(), anyhow::Error> {
    let block_id = fork_config.block_number.map_or(BlockId::Tag(BlockTag::Latest), BlockId::Number);

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

    check_forking_spec_version(json_rpc_client).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    configure_tracing();

    // parse arguments
    let args = Args::parse();
    let (mut starknet_config, server_config) = args.to_config()?;

    // If fork url is provided, then set fork config and chain_id from forked network
    if let Some(url) = starknet_config.fork_config.url.as_ref() {
        let json_rpc_client = JsonRpcClient::new(HttpTransport::new(url.clone()));
        set_and_log_fork_config(&mut starknet_config.fork_config, &json_rpc_client).await?;

        starknet_config.chain_id = json_rpc_client.chain_id().await?.into();
    }

    let address = format!("{}:{}", server_config.host, server_config.port);
    let listener = TcpListener::bind(address.clone()).await?;

    let api = Api::new(Starknet::new(&starknet_config)?);

    // set block timestamp shift during startup if start time is set
    if let Some(start_time) = starknet_config.start_time {
        api.starknet.lock().await.set_block_timestamp_shift(
            start_time as i64 - Starknet::get_unix_timestamp_as_seconds() as i64,
        );
    };

    log_predeployed_contracts();
    log_chain_id(&starknet_config.chain_id);

    let predeployed_accounts = api.starknet.lock().await.get_predeployed_accounts();
    log_predeployed_accounts(
        &predeployed_accounts,
        starknet_config.seed,
        starknet_config.predeployed_accounts_initial_balance.clone(),
    );

    // check restricted methods of server_config
    if let Some(restricted_methods) = server_config.restricted_methods.as_ref() {
        assert_all_restricted_methods_correct(restricted_methods)?;
    }

    let server = serve_http_api_json_rpc(listener, api.clone(), &starknet_config, &server_config);

    info!("Starknet Devnet listening on {}", address);

    let mut tasks = vec![];

    if let BlockGenerationOn::Interval(seconds) = starknet_config.block_generation_on {
        // use JoinHandle to run block interval creation as a task
        let block_interval_handle = task::spawn(create_block_interval(api.clone(), seconds));

        tasks.push(block_interval_handle);
    }

    // run server also as a JoinHandle
    let server_handle =
        task::spawn(server.with_graceful_shutdown(shutdown_signal(api.clone())).into_future());
    tasks.push(server_handle);

    // wait for ctrl + c signal (SIGINT)
    shutdown_signal(api.clone()).await;

    // join all tasks
    let results = join_all(tasks).await;

    // handle the results of the tasks
    for result in results {
        result??;
    }

    Ok(())
}

async fn create_block_interval(
    api: Api,
    block_interval_seconds: u64,
) -> Result<(), std::io::Error> {
    let mut interval = interval(Duration::from_secs(block_interval_seconds));

    #[cfg(unix)]
    let mut sigint = { signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler") };

    #[cfg(windows)]
    let mut sigint = {
        let ctrl_c_signal = ctrl_c().expect("Failed to setup Ctrl+C handler");
        Box::pin(ctrl_c_signal)
    };

    loop {
        // avoid creating block instantly after startup
        sleep(Duration::from_secs(block_interval_seconds)).await;

        tokio::select! {
            _ = interval.tick() => {
                let mut starknet = api.starknet.lock().await;
                info!("Generating block on time interval");

                starknet.create_block_dump_event(None).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
            }
            _ = sigint.recv() => {
                return Ok(())
            }
        }
    }
}

pub async fn shutdown_signal(api: Api) {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");

    // dump on exit scenario
    let starknet = api.starknet.lock().await;
    if starknet.config.dump_on == Some(DumpOn::Exit) {
        starknet.dump_events().expect("Failed to dump starknet transactions");
    }
}

fn assert_all_restricted_methods_correct(
    restricted_methods: &Vec<String>,
) -> Result<(), anyhow::Error> {
    let json_rpc_methods = JsonRpcRequest::all_variants_serde_renames();
    let all_methods: HashSet<_> = HashSet::from_iter(
        json_rpc_methods.iter().chain(HTTP_API_ROUTES_WITHOUT_LEADING_SLASH.iter()),
    );

    for restricted_method in restricted_methods {
        if !all_methods.contains(restricted_method) {
            let error_msg = "Restricted methods contain JSON-RPC methods and/or HTTP routes that \
                             are not supported by the server.";
            error!("{}", error_msg);
            anyhow::bail!(error_msg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::EnvFilter;

    use crate::{assert_all_restricted_methods_correct, configure_tracing};

    #[test]
    fn test_generated_log_level_from_empty_environment_variable_is_info() {
        assert_environment_variable_sets_expected_log_level("", LevelFilter::INFO);
    }

    #[test]
    fn check_if_method_with_incorrect_name_will_produce_an_error() {
        let err = assert_all_restricted_methods_correct(
            &["devnet_dump".to_string(), "devnet_loadd".to_string()].to_vec(),
        )
        .unwrap_err();

        assert!(err.to_string().contains(
            "Restricted methods contain JSON-RPC methods and/or HTTP routes that are not \
             supported by the server."
        ));
    }

    #[test]
    fn check_if_methods_with_correct_names_will_not_produce_an_error() {
        assert_all_restricted_methods_correct(
            &["devnet_dump".to_string(), "devnet_load".to_string()].to_vec(),
        )
        .unwrap();
    }

    fn assert_environment_variable_sets_expected_log_level(
        env_var: &str,
        expected_level: LevelFilter,
    ) {
        std::env::set_var(EnvFilter::DEFAULT_ENV, env_var);
        configure_tracing();

        assert_eq!(LevelFilter::current(), expected_level);
    }
}
