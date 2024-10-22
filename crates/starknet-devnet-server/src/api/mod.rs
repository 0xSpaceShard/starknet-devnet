pub mod http;
pub mod json_rpc;
pub mod serde_helpers;

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use starknet_core::starknet::Starknet;
use tokio::sync::Mutex;

use crate::dump_util::DumpEvent;

type SubscriptionId = u64;

/// Data that can be shared between threads with read write lock access
/// Whatever needs to be accessed as information outside of Starknet could be added to this struct
#[derive(Clone)]
pub struct Api {
    // maybe the config should be added here next to the starknet instance
    pub starknet: Arc<Mutex<Starknet>>,
    pub dumpable_events: Arc<Mutex<Vec<DumpEvent>>>,
    // TODO temporary set message type to u32, of course it shall be something more complex
    pub starknet_event_senders: Arc<Mutex<HashMap<SubscriptionId, Sender<u32>>>>,
}

impl Api {
    pub fn new(starknet: Starknet) -> Self {
        Self {
            starknet: Arc::new(Mutex::new(starknet)),
            dumpable_events: Default::default(),
            starknet_event_senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
