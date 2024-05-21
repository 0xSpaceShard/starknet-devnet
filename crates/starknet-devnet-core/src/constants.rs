use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU128;

use nonzero_ext::nonzero;
use starknet_types::chain_id::ChainId;

pub const CAIRO_0_ACCOUNT_CONTRACT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/accounts_artifacts/OpenZeppelin/0.5.1/Account.cairo/Account.casm"
);

pub const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
    "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/accounts_artifacts/OpenZeppelin/0.8.1/Account.cairo/Account.sierra"
);

pub const CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH: &str =
    "0x061dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f";

pub const CAIRO_1_ERC20_CONTRACT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/ERC20_Mintable_OZ_0.8.1.json");

/// ERC20 class hash is hardcoded to be the same as OZ class hash ERC20.cairo although it should be
/// different, due to commented key attributes in struct Approval (owner and spender), and add of
/// mintable feature: https://docs.openzeppelin.com/contracts-cairo/0.8.1/presets
pub const CAIRO_1_ERC20_CONTRACT_CLASS_HASH: &str =
    "0x046ded64ae2dead6448e247234bab192a9c483644395b66f2155f2614e5804b0";

pub const CAIRO_0_ERC20_CONTRACT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/ERC20_Mintable_OZ_0.2.0.json");

pub const CAIRO_0_ERC20_CONTRACT_CLASS_HASH: &str =
    "0x6A22BF63C7BC07EFFA39A25DFBD21523D211DB0100A0AFD054D172B81840EAF";

pub const ETH_ERC20_CONTRACT_ADDRESS: &str =
    "0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7";

pub const STRK_ERC20_CONTRACT_ADDRESS: &str =
    "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d";

pub(crate) const UDC_CONTRACT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/UDC_OZ_0.5.0.json");

pub const UDC_CONTRACT_CLASS_HASH: &str =
    "0x7B3E05F48F0C69E4A65CE5E076A66271A527AFF2C34CE1083EC6E1526997A69";

pub const UDC_CONTRACT_ADDRESS: &str =
    "0x41A78E741E5AF2FEC34B695679BC6891742439F7AFB8484ECD7766661AD02BF";

pub const STARKNET_VERSION: &str = "0.13.1.1";

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
pub const DEVNET_DEFAULT_GAS_PRICE: NonZeroU128 = nonzero!(100_000_000_000u128);
pub const DEVNET_DEFAULT_DATA_GAS_PRICE: NonZeroU128 = nonzero!(100_000_000_000u128);
pub const DEVNET_DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const DEVNET_DEFAULT_PORT: u16 = 5050;
pub const DEVNET_DEFAULT_TIMEOUT: u16 = 120;
pub const DEVNET_DEFAULT_CHAIN_ID: ChainId = ChainId::Testnet;
pub const DEVNET_DEFAULT_STARTING_BLOCK_NUMBER: u64 = 0;
pub const DEVNET_DEFAULT_REQUEST_BODY_SIZE_LIMIT: usize = 2_000_000;

pub const SUPPORTED_TX_VERSION: u32 = 1;

// chargeable account
pub const CHARGEABLE_ACCOUNT_PUBLIC_KEY: &str =
    "0x4C37AB4F0994879337BFD4EAD0800776DB57DA382B8ED8EFAA478C5D3B942A4";
pub const CHARGEABLE_ACCOUNT_PRIVATE_KEY: &str = "0x5FB2959E3011A873A7160F5BB32B0ECE";
pub const CHARGEABLE_ACCOUNT_ADDRESS: &str =
    "0x1CAF2DF5ED5DDE1AE3FAEF4ACD72522AC3CB16E23F6DC4C7F9FAED67124C511";
