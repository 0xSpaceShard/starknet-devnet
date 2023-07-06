pub(crate) mod http;
pub(crate) mod json_rpc;
pub(crate) mod models;
pub(crate) mod serde_helpers;

use starknet_core::{Starknet};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Data that can be shared between threads with read write lock access
#[derive(Clone)]
pub struct Api {
    pub data: Arc<RwLock<Vec<u32>>>,
    pub starknet: Arc<RwLock<Starknet>>,
}

impl Api {
    pub fn new(starknet: Starknet) -> Self {
        Self { data: Arc::new(RwLock::new(Vec::new())), starknet: Arc::new(RwLock::new(starknet)) }
    }
}
