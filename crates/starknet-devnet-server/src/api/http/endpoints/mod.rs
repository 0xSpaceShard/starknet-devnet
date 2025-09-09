use serde::Serialize;
use starknet_core::starknet::starknet_config::StarknetConfig;

use crate::ServerConfig;

/// Accounts
pub mod accounts;
/// Dumping and loading
pub mod dump_load;
/// Mint token - Local faucet
pub mod mint_token;
/// Postman
pub mod postman;
/// Time
pub mod time;

#[derive(Serialize)]
pub struct DevnetConfig {
    #[serde(flatten)]
    pub(crate) starknet_config: StarknetConfig,
    pub(crate) server_config: ServerConfig,
}
