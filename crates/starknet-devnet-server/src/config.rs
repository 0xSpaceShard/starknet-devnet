use std::net::IpAddr;

use serde::Serialize;

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
