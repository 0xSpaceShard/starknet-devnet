use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub timeout: u16,
    pub request_body_size_limit: usize,
}
