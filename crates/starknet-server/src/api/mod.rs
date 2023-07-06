pub(crate) mod http;
pub(crate) mod json_rpc;
pub(crate) mod models;
pub(crate) mod serde_helpers;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Example of a data that can be shared between threads with read write lock access
#[derive(Clone)]
pub struct Api {
    pub data: Arc<RwLock<Vec<u32>>>,
}

impl Api {
    pub fn new() -> Self {
        Self { data: Arc::new(RwLock::new(Vec::new())) }
    }
}
