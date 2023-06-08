use clap::Parser;
use starknet_core::StarknetConfig;
use starknet_types::traits::ToHexString;

/// Run a local instance of Starknet Devnet
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    /// Number of accounts
    #[arg(long = "accounts")]
    #[arg(value_name = "ACCOUNTS")]
    #[arg(default_value = "10")]
    #[arg(help = "Specify the number of accounts to be predeployed;")]
    accounts_count: u8,

    /// initial balance of predeployed accounts
    #[arg(long = "initial-balance")]
    #[arg(short)]
    #[arg(value_name = "INITIAL_BALANCE")]
    #[arg(default_value = "1000000000000000000000")]
    #[arg(help = "Specify the initial balance of accounts to be predeployed;")]
    e: u128,

    // seed for predeployed accounts
    #[arg(long = "seed")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed;")]
    seed: Option<String>
}

impl Args {
    fn to_starknet_config(&self) -> starknet_core::StarknetConfig {
        StarknetConfig{
            seed: u32::from_str_radix(self.seed.clone().unwrap_or("123".to_string()).as_str(), 10).unwrap(),
            total_accounts: self.accounts_count,
            predeployed_accounts_initial_balance: self.e
        }
    }
}

fn main() {
    let args = Args::parse().to_starknet_config();
    
    let starknet = starknet_core::Starknet::new(&args).unwrap();
    let predeployed_accounts = starknet.get_predeployed_accounts();
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
}