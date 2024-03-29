use starknet_core::constants::DEVNET_DEFAULT_INITIAL_BALANCE;
use starknet_rs_core::types::FieldElement;

pub const HOST: &str = "localhost";
pub const MIN_PORT: u16 = 1025;
pub const MAX_PORT: u16 = 65_535;
pub const SEED: usize = 42;
pub const ACCOUNTS: usize = 3;
pub const CHAIN_ID: FieldElement = starknet_rs_core::chain_id::SEPOLIA;
pub const CHAIN_ID_CLI_PARAM: &str = "TESTNET";

// URL paths
pub const RPC_PATH: &str = "/rpc";
pub const HEALTHCHECK_PATH: &str = "/is_alive";

// predeployed account info with seed=42
pub const PREDEPLOYED_ACCOUNT_ADDRESS: &str =
    "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba";
pub const PREDEPLOYED_ACCOUNT_PRIVATE_KEY: &str = "0xb137668388dbe9acdfa3bc734cc2c469";
pub const PREDEPLOYED_ACCOUNT_PUBLIC_KEY: &str =
    "0x05a5e37c60e77a0318643b111f88413a76af6233c891a0cfb2804106372006d4";
// half the default value - sanity check
pub const PREDEPLOYED_ACCOUNT_INITIAL_BALANCE: u128 = DEVNET_DEFAULT_INITIAL_BALANCE / 2;

// Example contract with basic storage read and write - Cairo 1
pub const CAIRO_1_CONTRACT_PATH: &str = "test_data/rpc/contract_cairo_v1/output.json";
pub const CAIRO_1_CASM_PATH: &str = "test_data/rpc/contract_cairo_v1/output-casm.json";
pub const CASM_COMPILED_CLASS_HASH: &str =
    "0x63b33a5f2f46b1445d04c06d7832c48c48ad087ce0803b71f2b8d96353716ca";

// Simple contract with a failable (panicking) function
pub const CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH: &str =
    "test_data/cairo1/panicking_contract/panicking_contract.sierra";

// Contract which asserts the tx version
pub const CAIRO_1_VERSION_ASSERTER_SIERRA_PATH: &str =
    "test_data/cairo1/version_asserter/version_asserter.sierra";

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
    "test_data/cairo1/invalid_account/invalid_account.sierra";
