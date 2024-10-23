use starknet_types::rpc::block::BlockId;
use tokio::sync::mpsc::Sender;

pub type SocketId = u64;
type SubscriptionId = u64;

struct NewHeadsSubscription {
    id: SubscriptionId,
    block_id: Option<BlockId>,
}

pub enum Subscription {
    NewHeads(NewHeadsSubscription),
    // TransactionStatus,
    // PendingTransactions,
    // Events,
    Reorg,
}

impl Subscription {
    // TODO
}

pub type SubscriptionResponse = u32;

pub struct SocketContext {
    /// The sender part of the socket's own channel
    starknet_sender: Sender<SubscriptionResponse>,
    subscriptions: Vec<Subscription>,
}

impl SocketContext {
    pub fn from_sender(sender: Sender<SubscriptionResponse>) -> Self {
        Self { starknet_sender: sender, subscriptions: vec![] }
    }
}
