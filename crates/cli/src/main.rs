use clap::Parser;
use starknet_core::StarknetConfig;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_types::traits::ToHexString;

/// Run a local instance of Starknet Devnet
#[derive(Parser, Debug)]
#[command(author, version, about="A Starknet devnet in Rust.", long_about = None)]
#[command(propagate_version = true)]
struct Args {
    /// Number of accounts.
    #[arg(long = "accounts")]
    #[arg(value_name = "ACCOUNTS")]
    #[arg(default_value = "10")]
    #[arg(help = "Specify the number of accounts to be predeployed;")]
    accounts_count: u8,

    /// Initial balance of predeployed accounts.
    #[arg(long = "initial-balance")]
    #[arg(short = 'e')]
    #[arg(value_name = "INITIAL_BALANCE")]
    #[arg(default_value = "1000000000000000000000")]
    #[arg(help = "Specify the initial balance of accounts to be predeployed;")]
    initial_balance: u128,

    // Seed for predeployed accounts.
    #[arg(long = "seed")]
    #[arg(value_name = "SEED")]
    #[arg(help = "Specify the seed for randomness of accounts to be predeployed;")]
    seed: Option<u32>,

    // Host address.
    #[arg(long = "host")]
    #[arg(value_name = "DEVNET_HOST")]
    #[arg(default_value = "127.0.0.1")]
    #[arg(help = "Specify the address to listen at;")]
    host: String,

    // Port number.
    #[arg(long = "port")]
    #[arg(value_name = "DEVNET_PORT")]
    #[arg(default_value = "5050")]
    #[arg(help = "Specify the port to listen at;")]
    port: u16,

    // Server timeout in seconds.
    #[arg(long = "timeout")]
    #[arg(value_name = "TIMEOUT")]
    #[arg(default_value = "120")]
    #[arg(help = "Specify the server timeout in seconds;")]
    timeout: u16,

    // Gas price in wei.
    #[arg(long = "gas-price")]
    #[arg(value_name = "GAS_PRICE")]
    #[arg(default_value = "100000000")]
    #[arg(help = "Specify the gas price in wei per gas unit;")]
    gas_price: u64,

    #[arg(long = "chain-id")]
    #[arg(value_name = "CHAIN_ID")]
    #[arg(default_value = "TESTNET")]
    #[arg(help = "Specify the chain id as one of: {MAINNET, TESTNET, TESTNET2};")]
    chain_id: String,
}

impl Args {
    fn to_starknet_config(&self) -> starknet_core::StarknetConfig {
        StarknetConfig {
            seed: match self.seed {
                Some(seed) => seed,
                None => random_number_generator::generate_u32_random_number(),
            },
            total_accounts: self.accounts_count,
            predeployed_accounts_initial_balance: self.initial_balance,
            host: self.host.clone(),
            port: self.port,
            timeout: self.timeout,
            gas_price: self.gas_price,
            chain_id: match self.chain_id.as_str() {
                "MAINNET" => StarknetChainId::MainNet,
                "TESTNET" => StarknetChainId::TestNet,
                "TESTNET2" => StarknetChainId::TestNet2,
                _ => StarknetChainId::TestNet,
            },
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let starknet_config = args.to_starknet_config();

    let starknet = starknet_core::Starknet::new(&starknet_config)?;
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

    starknet_server::start_server(&starknet_config).await?;
    Ok(())
}
