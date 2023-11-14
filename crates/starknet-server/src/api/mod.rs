pub(crate) mod http;
pub(crate) mod json_rpc;
pub(crate) mod serde_helpers;

use std::sync::Arc;

use starknet_core::starknet::Starknet;
use tokio::sync::RwLock;

/// Data that can be shared between threads with read write lock access
/// Whatever needs to be accessed as information outside of Starknet could be added to this struct
#[derive(Clone)]
pub struct Api {
    // maybe the config should be added here next to the starknet instance
    pub starknet: Arc<RwLock<Starknet>>,
}

impl Api {
    pub fn new(starknet: Starknet) -> Self {
        Self { starknet: Arc::new(RwLock::new(starknet)) }
    }
}
