use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub timeout: u16,
    pub request_body_size_limit: usize,
    #[serde(skip)]
    pub log_request: bool,
    #[serde(skip)]
    pub log_response: bool,
}
