use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU128;

use nonzero_ext::nonzero;
use starknet_rs_core::types::Felt;
use starknet_types::chain_id::ChainId;
use starknet_types::num_bigint::BigUint;

pub const CAIRO_0_ACCOUNT_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/0.5.1/Account.cairo/Account.json"
));
pub const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
    "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

/// only used in tests; if artifact needed in production, use CAIRO_1_ACCOUNT_CONTRACT_SIERRA
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/1.0.0/Account.cairo/Account.sierra"
);
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/1.0.0/Account.cairo/Account.sierra"
));
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH: &str =
    "0x05b4b537eaa2399e3aa99c4e2e0208ebd6c71bc1467938cd52c798c601e43564";

pub const ETH_ERC20_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x9524a94b41c4440a16fd96d7c1ef6ad6f44c1c013e96662734502cd4ee9b1f");
pub const ETH_ERC20_CONTRACT_CLASS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/system_artifacts/erc20_eth.sierra"
));
pub const ETH_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7");

pub const STRK_ERC20_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x76791ef97c042f81fbf352ad95f39a22554ee8d7927b2ce3c681f3418b5206a");
pub const STRK_ERC20_CONTRACT_CLASS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/system_artifacts/erc20_strk.sierra"
));
pub const STRK_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d");

// ERC20 contracts storage variables; available in source at https://github.com/starknet-io/starkgate-contracts
pub const ETH_ERC20_NAME: &str = "Ether";
pub const ETH_ERC20_SYMBOL: &str = "ETH";
pub const STRK_ERC20_NAME: &str = "StarkNet Token";
pub const STRK_ERC20_SYMBOL: &str = "STRK";

pub(crate) const UDC_LEGACY_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/system_artifacts/UDC_OZ_0.5.0.json"
));
pub const UDC_LEGACY_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x7B3E05F48F0C69E4A65CE5E076A66271A527AFF2C34CE1083EC6E1526997A69");
pub const UDC_LEGACY_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x41A78E741E5AF2FEC34B695679BC6891742439F7AFB8484ECD7766661AD02BF");

pub(crate) const UDC_CONTRACT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/system_artifacts/udc_2.sierra"));
pub const UDC_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x01b2df6d8861670d4a8ca4670433b2418d78169c2947f46dc614e69f333745c8");
pub const UDC_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x02ceed65a4bd731034c01113685c831b01c15d7d432f71afb1cf1634b53a2125");

/// https://github.com/argentlabs/argent-contracts-starknet/blob/main/deployments/account.txt
pub const ARGENT_CONTRACT_VERSION: &str = "0.4.0";
pub(crate) const ARGENT_CONTRACT_SIERRA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/Argent/argent_0.4.0.sierra"
));
pub const ARGENT_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x36078334509b514626504edc9fb252328d1a240e4e948bef8d0c08dff45927f");

/// https://github.com/argentlabs/argent-contracts-starknet/blob/main/deployments/multisig.txt
pub const ARGENT_MULTISIG_CONTRACT_VERSION: &str = "0.2.0";
pub(crate) const ARGENT_MULTISIG_CONTRACT_SIERRA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/Argent/argent_multisig_0.2.0.sierra"
));
pub const ARGENT_MULTISIG_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x7aeca3456816e3b833506d7cc5c1313d371fbdb0ae95ee70af72a4ddbf42594");

/// https://github.com/OpenZeppelin/cairo-contracts/blob/89a450a88628ec3b86273f261b2d8d1ca9b1522b/src/account/interface.cairo#L7
pub const ISRC6_ID_HEX: &str = "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd";

pub const STARKNET_VERSION: &str = "0.14.0";

pub const DEVNET_DEFAULT_SEED: u32 = 123;
pub const DEVNET_DEFAULT_TEST_SEED: u32 = 123;
pub const DEVNET_DEFAULT_TOTAL_ACCOUNTS: u8 = 10;
pub const DEVNET_DEFAULT_INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000_000;
pub const DEVNET_DEFAULT_L1_GAS_PRICE: NonZeroU128 = nonzero!(1_000_000_000u128);
pub const DEVNET_DEFAULT_L1_DATA_GAS_PRICE: NonZeroU128 = nonzero!(1_000_000_000u128);
pub const DEVNET_DEFAULT_L2_GAS_PRICE: NonZeroU128 = nonzero!(1_000_000_000u128);
pub const DEVNET_DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const DEVNET_DEFAULT_PORT: u16 = 5050;
pub const DEVNET_DEFAULT_TIMEOUT: u16 = 120;
pub const DEVNET_DEFAULT_CHAIN_ID: ChainId = ChainId::Testnet;
pub const DEVNET_DEFAULT_STARTING_BLOCK_NUMBER: u64 = 0;

pub const USE_KZG_DA: bool = true;

// chargeable account
pub const CHARGEABLE_ACCOUNT_PUBLIC_KEY: &str =
    "0x4C37AB4F0994879337BFD4EAD0800776DB57DA382B8ED8EFAA478C5D3B942A4";
pub const CHARGEABLE_ACCOUNT_PRIVATE_KEY: &str = "0x5FB2959E3011A873A7160F5BB32B0ECE";
pub const CHARGEABLE_ACCOUNT_ADDRESS: &str =
    "0x1CAF2DF5ED5DDE1AE3FAEF4ACD72522AC3CB16E23F6DC4C7F9FAED67124C511";
pub fn chargeable_account_initial_balance() -> BigUint {
    // Ideally, this would be a constant, but defining it as a string introduces parsing issues and
    // making lazy_static a dependency seems too much.
    BigUint::from(1_u32) << 255
}

pub const ENTRYPOINT_NOT_FOUND_ERROR_ENCODED: Felt =
    Felt::from_hex_unchecked("0x454e545259504f494e545f4e4f545f464f554e44");

pub const MAXIMUM_CONTRACT_CLASS_SIZE: u64 = 4_089_446;
pub const MAXIMUM_CONTRACT_BYTECODE_SIZE: u64 = 81_920;
pub const MAXIMUM_SIERRA_LENGTH: u64 = 81_920;
