use std::num::NonZeroU128;

use clap::Parser;
use starknet_core::constants::{
    DEVNET_DEFAULT_DATA_GAS_PRICE, DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_PORT,
    DEVNET_DEFAULT_TIMEOUT, DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};
use starknet_core::contract_class_choice::{AccountClassWrapper, AccountContractClassChoice};
use starknet_core::random_number_generator::generate_u32_random_number;
use starknet_core::starknet::starknet_config::{
    DumpOn, ForkConfig, StarknetConfig, StateArchiveCapacity,
};
use starknet_types::chain_id::ChainId;

use crate::initial_balance_wrapper::InitialBalanceWrapper;
use crate::ip_addr_wrapper::IpAddrWrapper;

/// Run a local instance of Starknet Devnet
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Args {
    /// Number of accounts
    #[arg(long = "accounts")]
    #[arg(value_name = "ACCOUNTS")]
    #[arg(default_value_t = DEVNET_DEFAULT_TOTAL_ACCOUNTS)]
    #[arg(help = "Specify the number of accounts to be predeployed;")]
    accounts_count: u8,

    /// Class used for account predeployment
    #[arg(long = "account-class")]
    #[arg(value_name = "ACCOUNT_CLASS")]
    #[arg(default_value = "cairo1")]
    #[arg(help = "Specify the class used by predeployed accounts;")]
    account_class_choice: AccountContractClassChoice,

    #[arg(long = "account-class-custom")]
    #[arg(value_name = "PATH")]
    #[arg(conflicts_with = "account_class_choice")]
    #[arg(help = "Specify the path to a Cairo Sierra artifact to be used by predeployed accounts;")]
    account_class_custom: Option<AccountClassWrapper>,

    /// Initial balance of predeployed accounts
    #[arg(long = "initial-balance")]
    #[arg(short = 'e')]
    #[arg(value_name = "DECIMAL_VALUE")]
    #[arg(default_value_t = InitialBalanceWrapper::default())]
    #[arg(help = "Specify the initial balance in WEI of accounts to be predeployed;")]
    initial_balance: InitialBalanceWrapper,

    // Seed for predeployed accounts
    #[arg(long = "seed")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed; if not \
                  provided, it is randomly generated")]
    seed: Option<u32>,

    // Host address
    #[arg(long = "host")]
    #[arg(value_name = "HOST")]
    #[arg(default_value_t = IpAddrWrapper::LOCALHOST)]
    #[arg(help = "Specify the address to listen at;")]
    host: IpAddrWrapper,

    // Port number
    #[arg(long = "port")]
    #[arg(value_name = "PORT")]
    #[arg(default_value_t = DEVNET_DEFAULT_PORT)]
    #[arg(help = "Specify the port to listen at;")]
    port: u16,

    // Set start time in seconds
    #[arg(long = "start-time")]
    #[arg(value_name = "START_TIME_IN_SECONDS")]
    #[arg(help = "Specify start time in seconds;")]
    start_time: Option<u64>,

    // Server timeout in seconds
    #[arg(long = "timeout")]
    #[arg(value_name = "TIMEOUT")]
    #[arg(default_value_t = DEVNET_DEFAULT_TIMEOUT)]
    #[arg(help = "Specify the server timeout in seconds;")]
    timeout: u16,

    // Gas price in wei
    #[arg(long = "gas-price")]
    #[arg(value_name = "GAS_PRICE")]
    #[arg(default_value_t = DEVNET_DEFAULT_GAS_PRICE)]
    #[arg(help = "Specify the gas price in wei per gas unit;")]
    gas_price: NonZeroU128,

    // Gas price in wei
    #[arg(long = "data-gas-price")]
    #[arg(value_name = "DATA_GAS_PRICE")]
    #[arg(default_value_t = DEVNET_DEFAULT_DATA_GAS_PRICE)]
    #[arg(help = "Specify the gas price in wei per data gas unit;")]
    data_gas_price: NonZeroU128,

    #[arg(long = "chain-id")]
    #[arg(value_name = "CHAIN_ID")]
    #[arg(default_value = "TESTNET")]
    #[arg(help = "Specify the chain ID;")]
    chain_id: ChainId,

    #[arg(long = "dump-on")]
    #[arg(value_name = "WHEN")]
    #[arg(help = "Specify when to dump the state of Devnet;")]
    #[arg(requires = "dump_path")]
    dump_on: Option<DumpOn>,

    // Dump path as string
    #[arg(long = "dump-path")]
    #[arg(value_name = "DUMP_PATH")]
    #[arg(help = "Specify the path to dump to;")]
    dump_path: Option<String>,

    #[arg(long = "state-archive-capacity")]
    #[arg(value_name = "STATE_ARCHIVE_CAPACITY")]
    #[arg(default_value = "none")]
    #[arg(help = "Specify the state archive capacity;")]
    state_archive: StateArchiveCapacity,

    #[arg(long = "fork-network")]
    #[arg(value_name = "URL")]
    #[arg(help = "Specify the URL of the network to fork;")]
    fork_network: Option<url::Url>,

    #[arg(long = "fork-block")]
    #[arg(value_name = "BLOCK_NUMBER")]
    #[arg(help = "Specify the number of the block to fork at;")]
    #[arg(requires = "fork_network")]
    fork_block: Option<u64>,
}

impl Args {
    pub(crate) fn to_starknet_config(&self) -> Result<StarknetConfig, anyhow::Error> {
        // use account-class-custom if specified; otherwise default to predefined account-class
        let account_class_wrapper = match &self.account_class_custom {
            Some(account_class_custom) => account_class_custom.clone(),
            None => self.account_class_choice.get_class_wrapper()?,
        };

        Ok(StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            account_contract_class: account_class_wrapper.contract_class,
            account_contract_class_hash: account_class_wrapper.class_hash,
            predeployed_accounts_initial_balance: self.initial_balance.0.clone(),
            host: self.host.inner,
            port: self.port,
            start_time: self.start_time,
            timeout: self.timeout,
            gas_price: self.gas_price,
            data_gas_price: self.data_gas_price,
            chain_id: self.chain_id,
            dump_on: self.dump_on,
            dump_path: self.dump_path.clone(),
            re_execute_on_init: true,
            state_archive: self.state_archive,
            fork_config: ForkConfig {
                url: self.fork_network.clone(),
                block_number: self.fork_block,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use starknet_core::constants::{
        CAIRO_0_ERC20_CONTRACT_PATH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
    };
    use starknet_core::starknet::starknet_config::StateArchiveCapacity;

    use super::Args;
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
        assert_eq!(args.to_starknet_config().unwrap().state_archive, StateArchiveCapacity::None);
    }

    #[test]
    fn state_archive_none() {
        let args = Args::parse_from(["--", "--state-archive-capacity", "none"]);
        assert_eq!(args.to_starknet_config().unwrap().state_archive, StateArchiveCapacity::None);
    }

    #[test]
    fn state_archive_full() {
        let args = Args::parse_from(["--", "--state-archive-capacity", "full"]);
        assert_eq!(args.to_starknet_config().unwrap().state_archive, StateArchiveCapacity::Full);
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
        let custom_path = "test_data/rpc/contract_cairo_v1/output.json";
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
}
