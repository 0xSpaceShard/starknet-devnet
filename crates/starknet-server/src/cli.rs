use clap::Parser;
use starknet_core::constants::{
    DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_PORT, DEVNET_DEFAULT_TIMEOUT,
    DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};
use starknet_core::starknet::starknet_config::{DumpOn, StarknetConfig};
use starknet_types::chain_id::ChainId;

use crate::contract_class_choice::{AccountClassWrapper, AccountContractClassChoice};
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
    #[arg(default_value = "cairo0")]
    #[arg(help = "Specify the class used by predeployed accounts;")]
    account_class: AccountContractClassChoice,

    #[arg(long = "account-class-path")]
    #[arg(value_name = "PATH")]
    #[arg(conflicts_with = "account_class")]
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
    gas_price: u64,

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
}

impl Args {
    pub(crate) fn to_starknet_config(&self) -> Result<StarknetConfig, anyhow::Error> {
        let (account_contract_class, account_contract_class_hash) = match &self.account_class_custom
        {
            Some(account_class_wrapper) => {
                (account_class_wrapper.contract_class.clone(), account_class_wrapper.class_hash)
            }
            None => (self.account_class.get_class()?, self.account_class.get_hash()?),
        };

        Ok(StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => random_number_generator::generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            account_contract_class,
            account_contract_class_hash,
            predeployed_accounts_initial_balance: self.initial_balance.0,
            host: self.host.inner,
            port: self.port,
            timeout: self.timeout,
            gas_price: self.gas_price,
            chain_id: self.chain_id,
            dump_on: self.dump_on,
            dump_path: self.dump_path.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use starknet_core::constants::CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH;

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
    fn not_allowing_account_class_and_account_class_path() {
        match Args::try_parse_from([
            "--",
            "--account-class",
            "cairo1",
            "--account-class-path",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ]) {
            Err(err) => {
                let err_str = err.to_string();
                let first_line = err_str.split('\n').next().unwrap();
                assert_eq!(
                    first_line,
                    "error: the argument '--account-class <ACCOUNT_CLASS>' cannot be used with \
                     '--account-class-path <PATH>'"
                );
            }
            Ok(parsed) => panic!("Should have failed; got: {parsed:?}"),
        }
    }

    #[test]
    fn allowing_only_account_class() {
        match Args::try_parse_from(["--", "--account-class", "cairo1"]) {
            Ok(_) => (),
            Err(err) => panic!("Should have passed; got: {err:?}"),
        }
    }

    #[test]
    fn allowing_only_account_class_path() {
        match Args::try_parse_from([
            "--",
            "--account-class-path",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ]) {
            Ok(_) => (),
            Err(err) => panic!("Should have passed; got: {err:?}"),
        }
    }
}
