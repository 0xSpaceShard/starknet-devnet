pub mod http;
pub mod json_rpc;
pub mod serde_helpers;

use std::collections::HashMap;
use std::sync::Arc;

use starknet_core::starknet::Starknet;
use tokio::sync::Mutex;

use crate::dump_util::DumpEvent;
use crate::subscribe::{SocketContext, SocketId};

/// Data that can be shared between threads with read write lock access
/// Whatever needs to be accessed as information outside of Starknet could be added to this struct
#[derive(Clone)]
pub struct Api {
    // maybe the config should be added here next to the starknet instance
    pub starknet: Arc<Mutex<Starknet>>,
    pub dumpable_events: Arc<Mutex<Vec<DumpEvent>>>,
    // TODO abstract sockets as SocketStorage instead of direct HashMap manipulation
    pub sockets: Arc<Mutex<HashMap<SocketId, SocketContext>>>,
}

impl Api {
    pub fn new(starknet: Starknet) -> Self {
        Self {
            starknet: Arc::new(Mutex::new(starknet)),
            dumpable_events: Default::default(),
            sockets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
