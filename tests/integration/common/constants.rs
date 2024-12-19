use starknet_rs_core::types::Felt;

pub const HOST: &str = "localhost";
pub const MIN_PORT: u16 = 1025;
pub const MAX_PORT: u16 = 65_535;
pub const SEED: usize = 42;
pub const ACCOUNTS: usize = 3;
pub const CHAIN_ID: Felt = starknet_rs_core::chain_id::SEPOLIA;

// URL paths
pub const RPC_PATH: &str = "/rpc";
pub const HEALTHCHECK_PATH: &str = "/is_alive";

// predeployed account info with seed=42
pub const PREDEPLOYED_ACCOUNT_ADDRESS: &str =
    "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba";

// half the default value - sanity check
pub const PREDEPLOYED_ACCOUNT_INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000_000 / 2;

// account classes
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH: &str =
    "0x061dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f";
pub const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
    "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/starknet-devnet-core/contracts/accounts_artifacts/OpenZeppelin/0.8.1/Account.\
     cairo/Account.sierra"
);
pub const CAIRO_1_ACCOUNT_CONTRACT_0_8_0_SIERRA_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/starknet-devnet-core/contracts/accounts_artifacts/OpenZeppelin/0.8.0/Account.\
     cairo/Account.sierra"
);

// system contracts
pub const CAIRO_1_ERC20_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x046ded64ae2dead6448e247234bab192a9c483644395b66f2155f2614e5804b0");
pub const ETH_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7");
pub const STRK_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d");
pub const UDC_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x41A78E741E5AF2FEC34B695679BC6891742439F7AFB8484ECD7766661AD02BF");

// Example contract with basic storage read and write - Cairo 1
pub const CAIRO_1_CONTRACT_PATH: &str =
    "../../contracts/test_artifacts/cairo1/simple_contract/output.sierra";

// Simple contract with a failable (panicking) function
pub const CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH: &str =
    "../../contracts/test_artifacts/cairo1/panicking_contract/panicking_contract.sierra";

// Contract which asserts the tx version
pub const CAIRO_1_VERSION_ASSERTER_SIERRA_PATH: &str =
    "../../contracts/test_artifacts/cairo1/version_asserter/version_asserter.sierra";

// L1 L2 pre-registered addresses.
/// Hardcoded in the cairo0 l1l2 contract
pub const MESSAGING_WHITELISTED_L1_CONTRACT: &str = "0x8359e4b0152ed5a731162d3c7b0d8d56edb165a0";

pub const L1_HANDLER_SELECTOR: &str =
    "0xc73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01";

pub const MESSAGING_L2_CONTRACT_ADDRESS: &str =
    "0x4db0679c568e6a9df6f21da9e243772853d6714b12b6b79c0551d9ea12ef91a";

pub const MESSAGING_L1_CONTRACT_ADDRESS: &str = "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512";

/// Cairo 1 account which panics on validation
pub const INVALID_ACCOUNT_SIERRA_PATH: &str =
    "../../contracts/test_artifacts/cairo1/invalid_account/invalid_account.sierra";

/// hash of the sierra artifact at commit d9f5220059c1e61ff87e4a5752522569135e464c of
/// argentlabs/argent-contracts-starknet:main
pub const ARGENT_ACCOUNT_CLASS_HASH: &str =
    "0x029927c8af6bccf3f6fda035981e765a7bdbf18a2dc0d630494f8758aa908e2b";

/// Forking
pub const INTEGRATION_SEPOLIA_HTTP_URL: &str =
    "http://rpc.pathfinder.equilibrium.co/integration-sepolia/rpc/v0_7";

pub const MAINNET_URL: &str = "http://rpc.pathfinder.equilibrium.co/mainnet/rpc/v0_7";
pub const MAINNET_HTTPS_URL: &str = "https://rpc.pathfinder.equilibrium.co/mainnet/rpc/v0_7";
pub const INTEGRATION_SEPOLIA_GENESIS_BLOCK_HASH: &str =
    "0x19f675d3fb226821493a6ab9a1955e384bba80f130de625621a418e9a7c0ca3";

// copied from starknet-rs, because it is not exposed as public type
pub const QUERY_VERSION_OFFSET: Felt =
    Felt::from_raw([576460752142434320, 18446744073709551584, 17407, 18446744073700081665]);
