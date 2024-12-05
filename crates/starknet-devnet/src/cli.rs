use std::collections::HashSet;
use std::num::NonZeroU128;

use clap::Parser;
use server::api::json_rpc::JsonRpcRequest;
use server::restrictive_mode::DEFAULT_RESTRICTED_JSON_RPC_METHODS;
use server::server::HTTP_API_ROUTES_WITHOUT_LEADING_SLASH;
use server::ServerConfig;
use starknet_core::constants::{
    DEVNET_DEFAULT_DATA_GAS_PRICE, DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_PORT,
    DEVNET_DEFAULT_REQUEST_BODY_SIZE_LIMIT, DEVNET_DEFAULT_TIMEOUT, DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};
use starknet_core::contract_class_choice::{AccountClassWrapper, AccountContractClassChoice};
use starknet_core::random_number_generator::generate_u32_random_number;
use starknet_core::starknet::starknet_config::{
    BlockGenerationOn, DumpOn, ForkConfig, StarknetConfig, StateArchiveCapacity,
};
use starknet_types::chain_id::ChainId;
use tracing_subscriber::EnvFilter;

use crate::initial_balance_wrapper::InitialBalanceWrapper;
use crate::ip_addr_wrapper::IpAddrWrapper;
use crate::{REQUEST_LOG_ENV_VAR, RESPONSE_LOG_ENV_VAR};

/// Run a local instance of Starknet Devnet
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A local testnet for Starknet... in Rust!",
    long_about = "Documentation: https://0xspaceshard.github.io/starknet-devnet-rs",
    propagate_version = true
)]
pub(crate) struct Args {
    /// Number of accounts
    #[arg(long = "accounts")]
    #[arg(env = "ACCOUNTS")]
    #[arg(value_name = "NUMBER")]
    #[arg(default_value_t = DEVNET_DEFAULT_TOTAL_ACCOUNTS)]
    #[arg(help = "Specify the number of accounts to be predeployed;")]
    accounts_count: u8,

    /// Class used for account predeployment
    #[arg(long = "account-class")]
    #[arg(env = "ACCOUNT_CLASS")]
    #[arg(value_name = "ACCOUNT_CLASS")]
    #[arg(default_value = "cairo1")]
    #[arg(help = "Specify the class used by predeployed accounts;")]
    account_class_choice: AccountContractClassChoice,

    #[arg(long = "account-class-custom")]
    #[arg(env = "ACCOUNT_CLASS_CUSTOM")]
    #[arg(value_name = "PATH")]
    #[arg(conflicts_with = "account_class_choice")]
    #[arg(help = "Specify the path to a Cairo Sierra artifact to be used by predeployed accounts;")]
    account_class_custom: Option<AccountClassWrapper>,

    #[arg(long = "predeclare-argent")]
    #[arg(env = "PREDECLARE_ARGENT")]
    #[arg(help = "If set, predeclares the latest Argent contract classes (regular and \
                  multisig); increases startup time several times;")]
    predeclare_argent: bool,

    /// Initial balance of predeployed accounts
    #[arg(long = "initial-balance")]
    #[arg(env = "INITIAL_BALANCE")]
    #[arg(short = 'e')]
    #[arg(value_name = "DECIMAL_VALUE")]
    #[arg(default_value_t = InitialBalanceWrapper::default())]
    #[arg(help = "Specify the initial balance in WEI of accounts to be predeployed;")]
    initial_balance: InitialBalanceWrapper,

    // Seed for predeployed accounts
    #[arg(long = "seed")]
    #[arg(env = "SEED")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed; if not \
                  provided, it is randomly generated")]
    seed: Option<u32>,

    // Host address
    #[arg(long = "host")]
    #[arg(env = "HOST")]
    #[arg(value_name = "HOST")]
    #[arg(default_value_t = IpAddrWrapper::LOCALHOST)]
    #[arg(help = "Specify the address to listen at;")]
    host: IpAddrWrapper,

    // Port number
    #[arg(long = "port")]
    #[arg(env = "PORT")]
    #[arg(value_name = "PORT")]
    #[arg(default_value_t = DEVNET_DEFAULT_PORT)]
    #[arg(help = "Specify the port to listen at;")]
    port: u16,

    // Set start time in seconds
    #[arg(long = "start-time")]
    #[arg(env = "START_TIME")]
    #[arg(value_name = "SECONDS")]
    #[arg(help = "Specify start time in seconds;")]
    start_time: Option<u64>,

    // Server timeout in seconds
    #[arg(long = "timeout")]
    #[arg(env = "TIMEOUT")]
    #[arg(value_name = "SECONDS")]
    #[arg(default_value_t = DEVNET_DEFAULT_TIMEOUT)]
    #[arg(help = "Specify the server timeout in seconds;")]
    timeout: u16,

    // Gas price in wei
    #[arg(long = "gas-price")]
    #[arg(env = "GAS_PRICE")]
    #[arg(value_name = "WEI_PER_GAS_UNIT")]
    #[arg(default_value_t = DEVNET_DEFAULT_GAS_PRICE)]
    #[arg(help = "Specify the gas price in wei per gas unit;")]
    gas_price_wei: NonZeroU128,

    // Gas price in fri
    #[arg(long = "gas-price-fri")]
    #[arg(env = "GAS_PRICE_FRI")]
    #[arg(value_name = "FRI_PER_GAS_UNIT")]
    #[arg(default_value_t = DEVNET_DEFAULT_GAS_PRICE)]
    #[arg(help = "Specify the gas price in fri per gas unit;")]
    gas_price_fri: NonZeroU128,

    // Gas price in wei
    #[arg(long = "data-gas-price")]
    #[arg(env = "DATA_GAS_PRICE")]
    #[arg(value_name = "WEI_PER_GAS_UNIT")]
    #[arg(default_value_t = DEVNET_DEFAULT_DATA_GAS_PRICE)]
    #[arg(help = "Specify the gas price in wei per data gas unit;")]
    data_gas_price_wei: NonZeroU128,

    // Gas price in fri
    #[arg(long = "data-gas-price-fri")]
    #[arg(env = "DATA_GAS_PRICE_FRI")]
    #[arg(value_name = "FRI_PER_GAS_UNIT")]
    #[arg(default_value_t = DEVNET_DEFAULT_DATA_GAS_PRICE)]
    #[arg(help = "Specify the gas price in fri per data gas unit;")]
    data_gas_price_fri: NonZeroU128,

    #[arg(long = "chain-id")]
    #[arg(env = "CHAIN_ID")]
    #[arg(value_name = "CHAIN_ID")]
    #[arg(default_value = "TESTNET")]
    #[arg(help = "Specify the chain ID. Possible values are:
- \"MAINNET\", \"TESTNET\" - predefined chain IDs
- <USER_SUPPLIED> - custom chain ID, defined by user. Have to contain only ASCII characters.")]
    #[arg(conflicts_with = "fork_network")]
    chain_id: ChainId,

    #[arg(long = "dump-on")]
    #[arg(env = "DUMP_ON")]
    #[arg(value_name = "EVENT")]
    #[arg(help = "Specify when to dump the state of Devnet;")]
    dump_on: Option<DumpOn>,

    #[arg(long = "lite-mode")]
    #[arg(env = "LITE_MODE")]
    #[arg(help = "Specify whether to run in lite mode and skip block hash calculation;")]
    lite_mode: bool,

    // Dump path as string
    #[arg(long = "dump-path")]
    #[arg(env = "DUMP_PATH")]
    #[arg(value_name = "DUMP_PATH")]
    #[arg(help = "Specify the path to dump to;")]
    #[arg(required_if_eq_any([("dump_on", "exit"), ("dump_on", "block")]))]
    dump_path: Option<String>,

    #[arg(long = "block-generation-on")]
    #[arg(env = "BLOCK_GENERATION_ON")]
    #[arg(default_value = "transaction")]
    #[arg(help = "Specify when to generate a new block. Possible values are:
- \"transaction\" - new block generated on each transaction
- \"demand\" - new block creatable solely by sending a POST request to /create_block
- <INTERVAL> - a positive integer indicating after how many seconds a new block is generated

Sending POST /create_block is also an option in modes other than \"demand\".")]
    block_generation_on: BlockGenerationOn,

    #[arg(long = "state-archive-capacity")]
    #[arg(env = "STATE_ARCHIVE_CAPACITY")]
    #[arg(value_name = "STATE_ARCHIVE_CAPACITY")]
    #[arg(default_value = "none")]
    #[arg(help = "Specify the state archive capacity;")]
    state_archive: StateArchiveCapacity,

    #[arg(long = "fork-network")]
    #[arg(env = "FORK_NETWORK")]
    #[arg(value_name = "URL")]
    #[arg(help = "Specify the URL of the network to fork;")]
    fork_network: Option<url::Url>,

    #[arg(long = "fork-block")]
    #[arg(env = "FORK_BLOCK")]
    #[arg(value_name = "BLOCK_NUMBER")]
    #[arg(help = "Specify the number of the block to fork at;")]
    #[arg(requires = "fork_network")]
    fork_block: Option<u64>,

    #[arg(long = "request-body-size-limit")]
    #[arg(env = "REQUEST_BODY_SIZE_LIMIT")]
    #[arg(value_name = "BYTES")]
    #[arg(help = "Specify the maximum HTTP request body size;")]
    #[arg(default_value_t = DEVNET_DEFAULT_REQUEST_BODY_SIZE_LIMIT)]
    request_body_size_limit: usize,

    #[arg(long = "restrictive-mode")]
    #[arg(env = "RESTRICTIVE_MODE")]
    #[arg(num_args = 0..)]
    #[arg(help = "Use Devnet in restrictive mode; You can specify the methods that will be \
                  forbidden with whitespace-separated values (https://0xspaceshard.github.io/starknet-devnet-rs/docs/restrictive#with-a-list-of-methods). If nothing is specified for this \
                  argument, then default restricted methods are used (https://0xspaceshard.github.io/starknet-devnet-rs/docs/restrictive#default-restricted-methods).")]
    restricted_methods: Option<Vec<String>>,
}

impl Args {
    pub(crate) fn to_config(&self) -> Result<(StarknetConfig, ServerConfig), anyhow::Error> {
        // use account-class-custom if specified; otherwise default to predefined account-class
        let account_class_wrapper = match &self.account_class_custom {
            Some(account_class_custom) => account_class_custom.clone(),
            None => self.account_class_choice.get_class_wrapper()?,
        };

        let starknet_config = StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            account_contract_class: account_class_wrapper.contract_class,
            account_contract_class_hash: account_class_wrapper.class_hash,
            predeployed_accounts_initial_balance: self.initial_balance.0.clone(),
            start_time: self.start_time,
            gas_price_wei: self.gas_price_wei,
            gas_price_fri: self.gas_price_fri,
            data_gas_price_wei: self.data_gas_price_wei,
            data_gas_price_fri: self.data_gas_price_fri,
            chain_id: self.chain_id,
            dump_on: self.dump_on,
            dump_path: self.dump_path.clone(),
            block_generation_on: self.block_generation_on,
            lite_mode: self.lite_mode,
            state_archive: self.state_archive,
            fork_config: ForkConfig {
                url: self.fork_network.clone(),
                block_number: self.fork_block,
                block_hash: None,
            },
            predeclare_argent: self.predeclare_argent,
            ..Default::default()
        };

        let RequestResponseLogging { log_request, log_response } =
            RequestResponseLogging::from_rust_log_environment_variable();

        // if restricted_methods are not specified, use default ones
        let restricted_methods = self.restricted_methods.as_ref().map(|methods| {
            if methods.is_empty() {
                DEFAULT_RESTRICTED_JSON_RPC_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            } else {
                // remove leading slashes
                methods
                    .iter()
                    .map(|s| s.trim_start_matches('/').to_string())
                    .collect::<Vec<String>>()
            }
        });
        // validate restricted methods
        if let Some(methods) = restricted_methods.as_ref() {
            let json_rpc_methods = JsonRpcRequest::all_variants_serde_renames();
            let all_methods: HashSet<_> = HashSet::from_iter(
                json_rpc_methods.iter().chain(HTTP_API_ROUTES_WITHOUT_LEADING_SLASH.iter()),
            );
            let mut wrong_restricted_methods = vec![];
            for method in methods {
                if !all_methods.contains(method) {
                    wrong_restricted_methods.push(method.clone());
                }
            }
            if !wrong_restricted_methods.is_empty() {
                anyhow::bail!(
                    "Restricted methods contain JSON-RPC methods and/or HTTP routes that are not \
                     supported by the server: {}",
                    wrong_restricted_methods.join(" ")
                );
            }
        }

        let server_config = ServerConfig {
            host: self.host.inner,
            port: self.port,
            timeout: self.timeout,
            request_body_size_limit: self.request_body_size_limit,
            log_request,
            log_response,
            restricted_methods,
        };

        Ok((starknet_config, server_config))
    }
}

struct RequestResponseLogging {
    log_request: bool,
    log_response: bool,
}

impl RequestResponseLogging {
    fn from_rust_log_environment_variable() -> Self {
        let log_env_var = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default().to_lowercase();
        let log_request = log_env_var.contains(REQUEST_LOG_ENV_VAR);
        let log_response = log_env_var.contains(RESPONSE_LOG_ENV_VAR);

        Self { log_request, log_response }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use starknet_core::constants::{
        CAIRO_0_ERC20_CONTRACT_PATH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
    };
    use starknet_core::starknet::starknet_config::{
        BlockGenerationOn, DumpOn, StateArchiveCapacity,
    };
    use tracing_subscriber::EnvFilter;

    use super::{Args, RequestResponseLogging};
    use crate::ip_addr_wrapper::IpAddrWrapper;

    #[test]
    fn valid_ip_address() {
        Args::parse_from(["--", "--host", "127.0.0.1"]);
    }

    #[test]
    fn invalid_ip_address() {
        let invalid_ip_address = "127.0.0";
        match Args::try_parse_from(["--", "--host", invalid_ip_address]) {
            Err(_) => (),
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn localhost_mapped() {
        let args = Args::parse_from(["--", "--host", "localhost"]);
        assert_eq!(args.host, IpAddrWrapper::LOCALHOST);
    }

    #[test]
    fn invalid_hostname() {
        match Args::try_parse_from(["--", "--host", "invalid"]) {
            Err(_) => (),
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn state_archive_default_none() {
        let args = Args::parse_from(["--"]);
        let (starknet_config, _) = args.to_config().unwrap();
        assert_eq!(starknet_config.state_archive, StateArchiveCapacity::None);
    }

    #[test]
    fn state_archive_none() {
        let args = Args::parse_from(["--", "--state-archive-capacity", "none"]);
        let (starknet_config, _) = args.to_config().unwrap();
        assert_eq!(starknet_config.state_archive, StateArchiveCapacity::None);
    }

    #[test]
    fn state_archive_full() {
        let args = Args::parse_from(["--", "--state-archive-capacity", "full"]);
        let (starknet_config, _) = args.to_config().unwrap();
        assert_eq!(starknet_config.state_archive, StateArchiveCapacity::Full);
    }

    fn get_first_line(text: &str) -> &str {
        text.split('\n').next().unwrap()
    }

    #[test]
    fn not_allowing_account_class_and_account_class_path() {
        match Args::try_parse_from([
            "--",
            "--account-class",
            "cairo1",
            "--account-class-custom",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ]) {
            Err(err) => {
                assert_eq!(
                    get_first_line(&err.to_string()),
                    "error: the argument '--account-class <ACCOUNT_CLASS>' cannot be used with \
                     '--account-class-custom <PATH>'"
                );
            }
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn allowing_if_only_account_class() {
        match Args::try_parse_from(["--", "--account-class", "cairo1"]) {
            Ok(_) => (),
            Err(err) => panic!("Should have passed; got: {err}"),
        }
    }

    #[test]
    fn allowing_if_only_account_class_path() {
        match Args::try_parse_from([
            "--",
            "--account-class-custom",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ]) {
            Ok(_) => (),
            Err(err) => panic!("Should have passed; got: {err}"),
        }
    }

    #[test]
    fn not_allowing_regular_cairo0_contract_as_custom_account() {
        let custom_path = CAIRO_0_ERC20_CONTRACT_PATH;
        match Args::try_parse_from(["--", "--account-class-custom", custom_path]) {
            Err(err) => assert_eq!(
                get_first_line(&err.to_string()),
                format!(
                    "error: invalid value '{custom_path}' for '--account-class-custom <PATH>': \
                     Types error: missing field `kind` at line 1 column 292"
                )
            ),
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn not_allowing_regular_cairo1_contract_as_custom_account() {
        // path to a regular cairo1 contract (not an account)
        let custom_path = "../../contracts/test_artifacts/cairo1/simple_contract/output.sierra";
        match Args::try_parse_from(["--", "--account-class-custom", custom_path]) {
            Err(err) => assert_eq!(
                get_first_line(&err.to_string()),
                format!(
                    "error: invalid value '{custom_path}' for '--account-class-custom <PATH>': \
                     Failed to load ContractClass: Not a valid Sierra account artifact; has \
                     __execute__: false; has __validate__: false"
                )
            ),
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn not_allowing_invalid_url_as_fork_network() {
        for url in ["abc", "", "http://"] {
            match Args::try_parse_from(["--", "--fork-network", url]) {
                Err(_) => (),
                Ok(parsed) => panic!("Should fail for {url}; got: {parsed:?}"),
            }
        }
    }

    #[test]
    fn allowing_valid_url_as_fork_network() {
        for url in [
            "https://free-rpc.nethermind.io/mainnet-juno/v0_6",
            "http://localhost/",
            "http://localhost:5051/",
            "http://127.0.0.1/",
            "http://127.0.0.1:5050/",
            "https://localhost/",
            "https://localhost:5050/",
        ] {
            match Args::try_parse_from(["--", "--fork-network", url]) {
                Ok(args) => assert_eq!(args.fork_network.unwrap().to_string(), url),
                Err(e) => panic!("Should have passed; got: {e}"),
            }
        }
    }

    #[test]
    fn not_allowing_fork_block_without_fork_network() {
        match Args::try_parse_from(["--", "--fork-block", "12"]) {
            Err(_) => (),
            Ok(parsed) => panic!("Should fail when just --fork-block got: {parsed:?}"),
        }
    }

    #[test]
    fn not_allowing_invalid_value_as_fork_block() {
        for number in ["", "abc", "-1"] {
            match Args::try_parse_from([
                "--",
                "--fork-network",
                "http://localhost:5051",
                "--fork-block",
                number,
            ]) {
                Err(_) => (),
                Ok(parsed) => panic!("Should fail for {number}; got: {parsed:?}"),
            }
        }
    }

    #[test]
    fn allowing_number_as_fork_block() {
        for number in [0, 1, 42, 999] {
            match Args::try_parse_from([
                "--",
                "--fork-network",
                "http://localhost:5051",
                "--fork-block",
                &number.to_string(),
            ]) {
                Ok(args) => assert_eq!(args.fork_block, Some(number)),
                Err(e) => panic!("Should have passed; got: {e}"),
            }
        }
    }

    #[test]
    fn allowing_big_positive_request_body_size() {
        let value = 1_000_000_000;
        match Args::try_parse_from(["--", "--request-body-size-limit", &value.to_string()]) {
            Ok(args) => assert_eq!(args.request_body_size_limit, value),
            Err(e) => panic!("Should have passed; got: {e}"),
        }
    }

    #[test]
    fn not_allowing_negative_request_body_size() {
        match Args::try_parse_from(["--", "--request-body-size-limit", "-1"]) {
            Err(_) => (),
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_variants_of_env_var() {
        for (environment_variable, should_log_request, should_log_response) in [
            ("request,response,info", true, true),
            ("request,info", true, false),
            ("response,info", false, true),
            ("info", false, false),
            ("", false, false),
            ("REQUEST,RESPONSE", true, true),
            ("REQUEST", true, false),
            ("RESPONSE", false, true),
        ] {
            std::env::set_var(EnvFilter::DEFAULT_ENV, environment_variable);
            let RequestResponseLogging { log_request, log_response } =
                RequestResponseLogging::from_rust_log_environment_variable();

            assert_eq!(log_request, should_log_request);
            assert_eq!(log_response, should_log_response);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_env_vars_have_same_effect_as_cli_params() {
        let config_source = [
            ("--accounts", "ACCOUNTS", "1"),
            ("--account-class", "ACCOUNT_CLASS", "cairo0"),
            ("--initial-balance", "INITIAL_BALANCE", "1"),
            ("--seed", "SEED", "43"),
            ("--port", "PORT", "1234"),
            ("--start-time", "START_TIME", "123"),
            ("--timeout", "TIMEOUT", "12"),
            ("--gas-price", "GAS_PRICE", "1"),
            ("--gas-price-fri", "GAS_PRICE_FRI", "2"),
            ("--data-gas-price", "DATA_GAS_PRICE", "3"),
            ("--data-gas-price-fri", "DATA_GAS_PRICE_FRI", "4"),
            ("--dump-on", "DUMP_ON", "exit"),
            ("--dump-path", "DUMP_PATH", "dummy-path"),
            ("--state-archive-capacity", "STATE_ARCHIVE_CAPACITY", "full"),
            ("--fork-network", "FORK_NETWORK", "http://dummy.com"),
            ("--fork-block", "FORK_BLOCK", "42"),
            ("--request-body-size-limit", "REQUEST_BODY_SIZE_LIMIT", "100"),
            ("--block-generation-on", "BLOCK_GENERATION_ON", "demand"),
        ];

        let mut cli_args = vec!["--"];
        for (cli_param, _, value) in config_source {
            cli_args.push(cli_param);
            cli_args.push(value);
        }

        let config_via_cli = Args::parse_from(cli_args).to_config().unwrap();

        for (_, var_name, value) in config_source {
            std::env::set_var(var_name, value);
        }
        let config_via_env = Args::parse_from(["--"]).to_config().unwrap();

        assert_eq!(
            serde_json::to_value(config_via_cli).unwrap(),
            serde_json::to_value(config_via_env).unwrap()
        );

        // remove var to avoid collision with other tests
        for (_, var_name, _) in config_source {
            std::env::remove_var(var_name);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_boolean_param_specification_via_env_vars() {
        let config_source =
            [("--lite-mode", "LITE_MODE"), ("--predeclare-argent", "PREDECLARE_ARGENT")];

        let mut cli_args = vec!["--"];
        for (cli_param, _) in config_source {
            cli_args.push(cli_param);
        }

        let mut config_via_cli =
            serde_json::to_value(Args::parse_from(cli_args).to_config().unwrap()).unwrap();

        for (_, var_name) in config_source {
            std::env::set_var(var_name, "true");
        }
        let mut config_via_env =
            serde_json::to_value(Args::parse_from(["--"]).to_config().unwrap()).unwrap();

        // Removing seed as it is generated randomly - it would make the compared objects different.
        // to_config returns two config parts, so using index 0 to address starknet_config.
        config_via_cli[0]["seed"].take();
        config_via_env[0]["seed"].take();

        assert_eq!(config_via_cli, config_via_env);

        // remove var to avoid collision with other tests
        for (var_name, _) in config_source {
            std::env::remove_var(var_name);
        }
    }

    #[test]
    fn not_allowing_invalid_values_as_block_generation_interval() {
        for interval in ["", "0", "-1", "abc"] {
            match Args::try_parse_from(["--", "--block-generation-on", interval]) {
                Err(_) => (),
                Ok(parsed) => panic!("Should fail for {interval}; got: {parsed:?}"),
            }
        }
    }

    #[test]
    fn allowing_valid_values_as_block_generation_interval() {
        match Args::try_parse_from(["--", "--block-generation-on", "1"]) {
            Ok(args) => assert_eq!(args.block_generation_on, BlockGenerationOn::Interval(1)),
            Err(e) => panic!("Should have passed; got: {e}"),
        }

        match Args::try_parse_from(["--", "--block-generation-on", "demand"]) {
            Ok(args) => assert_eq!(args.block_generation_on, BlockGenerationOn::Demand),
            Err(e) => panic!("Should have passed; got: {e}"),
        }

        match Args::try_parse_from(["--", "--block-generation-on", "transaction"]) {
            Ok(args) => assert_eq!(args.block_generation_on, BlockGenerationOn::Transaction),
            Err(e) => panic!("Should have passed; got: {e}"),
        }
    }

    #[test]
    fn test_when_dump_path_flag_required() {
        for event in ["exit", "block"] {
            match Args::try_parse_from(["--", "--dump-on", event]) {
                Ok(args) => panic!("Should have failed; got: {args:?}"),
                Err(e) => assert_eq!(
                    get_first_line(&e.to_string()),
                    "error: the following required arguments were not provided:"
                ),
            }
        }
        match Args::try_parse_from(["--", "--dump-on", "request"]) {
            Ok(Args { dump_on: Some(DumpOn::Request), .. }) => (),
            other => panic!("Invalid arg parse result: {other:?}"),
        }
    }

    #[test]
    fn invalid_dump_path_not_allowed() {
        match Args::try_parse_from(["--", "--dump-path", "dump_wrong_cli_mode", "--dump-on", "e"]) {
            Ok(args) => panic!("Should have failed; got: {args:?}"),
            Err(e) => assert_eq!(
                get_first_line(&e.to_string()),
                "error: invalid value 'e' for '--dump-on <EVENT>'"
            ),
        }
    }

    #[test]
    fn check_if_method_with_incorrect_name_will_produce_an_error() {
        let args = Args::parse_from(["--", "--restrictive-mode", "devnet_dump", "devnet_loadd"]);
        let err = args.to_config().unwrap_err();
        assert!(err.to_string().contains(
            "Restricted methods contain JSON-RPC methods and/or HTTP routes that are not \
             supported by the server: devnet_loadd"
        ));
    }

    #[test]
    fn check_if_methods_with_correct_names_will_not_produce_an_error() {
        Args::parse_from(["--", "--restrictive-mode"]).to_config().unwrap();

        Args::parse_from(["--", "--restrictive-mode", "devnet_dump", "/mint"]).to_config().unwrap();
    }
}
