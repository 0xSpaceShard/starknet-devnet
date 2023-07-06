use clap::Parser;
use starknet_core::StarknetConfig;

/// Run a local instance of Starknet Devnet
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Args {
    /// Number of accounts
    #[arg(long = "accounts")]
    #[arg(value_name = "ACCOUNTS")]
    #[arg(default_value = "10")]
    #[arg(help = "Specify the number of accounts to be predeployed;")]
    accounts_count: u8,

    /// initial balance of predeployed accounts
    #[arg(long = "initial-balance")]
    #[arg(short = 'e')]
    #[arg(value_name = "INITIAL_BALANCE")]
    #[arg(default_value = "1000000000000000000000")]
    #[arg(help = "Specify the initial balance of accounts to be predeployed;")]
    initial_balance: u128,

    // seed for predeployed accounts
    #[arg(long = "seed")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed;")]
    seed: Option<u32>,
}

impl Args {
    pub(crate) fn to_starknet_config(&self) -> StarknetConfig {
        StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => random_number_generator::generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            predeployed_accounts_initial_balance: self.initial_balance,
        }
    }
}