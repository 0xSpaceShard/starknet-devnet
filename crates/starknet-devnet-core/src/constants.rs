use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU128;

use nonzero_ext::nonzero;
use starknet_rs_core::types::Felt;
use starknet_types::chain_id::ChainId;

pub const CAIRO_0_ACCOUNT_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/0.5.1/Account.cairo/Account.json"
));

pub const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
    "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

/// only used in tests; if artifact needed in production, use CAIRO_1_ACCOUNT_CONTRACT_SIERRA
pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/0.20.0/Account.cairo/Account.sierra"
);

pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/accounts_artifacts/OpenZeppelin/0.20.0/Account.cairo/Account.sierra"
));

pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH: &str =
    "0x044cab2e6a3a7bc516425d06d76c6ffd56ae308864dbc66f8e75028e3784aa29";

pub const CAIRO_1_ERC20_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/system_artifacts/ERC20_Mintable_OZ_0.20.0.sierra"
));

/// Unlike in previous Devnet versions, now using the actual hash of the predeployed artifact
pub const CAIRO_1_ERC20_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x011374319a6e07b4f2738fa3bfa8cf2181bfb0dbb4d800215baa87b83a57877e");

/// only used in tests; if artifact needed in production, add a new constant that uses include_str!
pub const CAIRO_0_ERC20_CONTRACT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/system_artifacts/ERC20_Mintable_OZ_0.2.0.json");

pub const ETH_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7");

pub const STRK_ERC20_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d");

pub(crate) const UDC_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/system_artifacts/UDC_OZ_0.5.0.json"
));

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

pub const UDC_CONTRACT_CLASS_HASH: Felt =
    Felt::from_hex_unchecked("0x7B3E05F48F0C69E4A65CE5E076A66271A527AFF2C34CE1083EC6E1526997A69");

pub const UDC_CONTRACT_ADDRESS: Felt =
    Felt::from_hex_unchecked("0x41A78E741E5AF2FEC34B695679BC6891742439F7AFB8484ECD7766661AD02BF");

/// https://github.com/OpenZeppelin/cairo-contracts/blob/89a450a88628ec3b86273f261b2d8d1ca9b1522b/src/account/interface.cairo#L7
pub const ISRC6_ID_HEX: &str = "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd";

pub const STARKNET_VERSION: &str = "0.13.2";

/// ERC20 contracts storage variables
/// taken from starkcan urls:
/// https://testnet.starkscan.co/token/0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7#read-write-contract
/// https://testnet.starkscan.co/contract/0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d#read-write-contract
pub const ETH_ERC20_NAME: &str = "Ether";
pub const ETH_ERC20_SYMBOL: &str = "ETH";
pub const STRK_ERC20_NAME: &str = "StarkNet Token";
pub const STRK_ERC20_SYMBOL: &str = "STRK";

pub const DEVNET_DEFAULT_SEED: u32 = 123;
pub const DEVNET_DEFAULT_TEST_SEED: u32 = 123;
pub const DEVNET_DEFAULT_TOTAL_ACCOUNTS: u8 = 10;
pub const DEVNET_DEFAULT_INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000_000;
pub const DEVNET_DEFAULT_L1_GAS_PRICE: NonZeroU128 = nonzero!(100_000_000_000u128);
pub const DEVNET_DEFAULT_L1_DATA_GAS_PRICE: NonZeroU128 = nonzero!(100_000_000_000u128);
pub const DEVNET_DEFAULT_L2_GAS_PRICE: NonZeroU128 = nonzero!(100_000_000_000u128);
pub const DEVNET_DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const DEVNET_DEFAULT_PORT: u16 = 5050;
pub const DEVNET_DEFAULT_TIMEOUT: u16 = 120;
pub const DEVNET_DEFAULT_CHAIN_ID: ChainId = ChainId::Testnet;
pub const DEVNET_DEFAULT_STARTING_BLOCK_NUMBER: u64 = 0;
pub const DEVNET_DEFAULT_REQUEST_BODY_SIZE_LIMIT: usize = 2_000_000;

pub const USE_KZG_DA: bool = true;

// chargeable account
pub const CHARGEABLE_ACCOUNT_PUBLIC_KEY: &str =
    "0x4C37AB4F0994879337BFD4EAD0800776DB57DA382B8ED8EFAA478C5D3B942A4";
pub const CHARGEABLE_ACCOUNT_PRIVATE_KEY: &str = "0x5FB2959E3011A873A7160F5BB32B0ECE";
pub const CHARGEABLE_ACCOUNT_ADDRESS: &str =
    "0x1CAF2DF5ED5DDE1AE3FAEF4ACD72522AC3CB16E23F6DC4C7F9FAED67124C511";

pub const ENTRYPOINT_NOT_FOUND_ERROR_ENCODED: Felt =
    Felt::from_hex_unchecked("0x454e545259504f494e545f4e4f545f464f554e44");
