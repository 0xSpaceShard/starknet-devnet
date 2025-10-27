pub mod serde_helpers;

use std::sync::Arc;

use starknet_core::starknet::Starknet;
use starknet_core::starknet::starknet_config::StarknetConfig;
use tokio::sync::Mutex;
use tracing::error;

use crate::ServerConfig;
use crate::dump_util::DumpEvent;
use crate::subscribe::SocketCollection;

mod account_helpers;
mod endpoints;
mod endpoints_ws;
pub mod error;
pub mod json_rpc_handler;
pub mod models;
pub(crate) mod origin_forwarder;
#[cfg(test)]
mod spec_reader;
mod write_endpoints;

pub use json_rpc_handler::JsonRpcHandler;
pub const RPC_SPEC_VERSION: &str = "0.10.0";

use error::ApiError;

/// Data that can be shared between threads with read write lock access
/// Whatever needs to be accessed as information outside of Starknet could be added to this struct
#[derive(Clone)]
pub struct Api {
    pub config: Arc<StarknetConfig>,
    pub server_config: Arc<ServerConfig>,
    pub starknet: Arc<Mutex<Starknet>>,
    pub dumpable_events: Arc<Mutex<Vec<DumpEvent>>>,
    pub sockets: Arc<Mutex<SocketCollection>>,
}

impl Api {
    pub fn new(starknet: Starknet, server_config: ServerConfig) -> Self {
        Self {
            config: Arc::new(starknet.config.clone()),
            server_config: Arc::new(server_config),
            starknet: Arc::new(Mutex::new(starknet)),
            dumpable_events: Default::default(),
            sockets: Arc::new(Mutex::new(SocketCollection::default())),
        }
    }
}
