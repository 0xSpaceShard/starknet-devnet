use std::net::IpAddr;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub timeout: u16,
    pub request_body_size_limit: usize,
}
