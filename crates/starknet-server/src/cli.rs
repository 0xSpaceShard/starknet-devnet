use clap::Parser;
use starknet_core::constants::{
    DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_PORT, DEVNET_DEFAULT_TIMEOUT,
    DEVNET_DEFAULT_TOTAL_ACCOUNTS, DEVNET_DEFAULT_HOST, DEVNET_DEFAULT_INITIAL_BALANCE,
};
use starknet_core::StarknetConfig;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_types::num_bigint::BigUint;

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

    /// Initial balance of predeployed accounts
    #[arg(long = "initial-balance")]
    #[arg(short = 'e')]
    #[arg(value_name = "INITIAL_BALANCE")]
    #[arg(default_value_t = BigUint::from(DEVNET_DEFAULT_INITIAL_BALANCE))]
    #[arg(help = "Specify the initial balance in WEI of accounts to be predeployed;")]
    initial_balance: BigUint,

    // Seed for predeployed accounts
    #[arg(long = "seed")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed; if not \
                  provided, it is randomly generated")]
    seed: Option<u32>,

    // Host address
    #[arg(long = "host")]
    #[arg(value_name = "HOST")]
    #[arg(default_value = DEVNET_DEFAULT_HOST)]
    #[arg(help = "Specify the address to listen at;")]
    host: String,

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

    // Chain id as string
    #[arg(long = "chain-id")]
    #[arg(value_name = "CHAIN_ID")]
    #[arg(default_value = "TESTNET")]
    #[arg(help = "Specify the chain id as one of: {MAINNET, TESTNET, TESTNET2};")]
    chain_id: String,
}

impl Args {
    pub(crate) fn to_starknet_config(&self) -> StarknetConfig {
        StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => random_number_generator::generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            predeployed_accounts_initial_balance: self
                .initial_balance
                .clone()
                .try_into()
                .expect("Invalid value for initial balance"), // TODO: Doesn't exit nicely.
            host: self.host.to_string(),
            port: self.port, // TODO: Unification of parsing messages for host and port.
            timeout: self.timeout,
            gas_price: self.gas_price,
            chain_id: match self.chain_id.as_str() {
                "MAINNET" => StarknetChainId::MainNet,
                "TESTNET" => StarknetChainId::TestNet,
                "TESTNET2" => StarknetChainId::TestNet2,
                _ => panic!("Invalid value for chain-id"),
            },
        }
    }
}
