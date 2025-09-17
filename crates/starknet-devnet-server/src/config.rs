use std::net::IpAddr;

use serde::Serialize;
use starknet_core::starknet::starknet_config::StarknetConfig;

#[derive(Debug, Clone, Serialize)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub timeout: u16,
    #[serde(skip)]
    pub log_request: bool,
    #[serde(skip)]
    pub log_response: bool,
    pub restricted_methods: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct DevnetConfig {
    #[serde(flatten)]
    pub(crate) starknet_config: StarknetConfig,
    pub(crate) server_config: ServerConfig,
}
