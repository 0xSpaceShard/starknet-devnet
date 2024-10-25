use serde::{self, Deserialize, Serialize};
use starknet_types::rpc::block::BlockHeader;
use tokio::sync::mpsc::Sender;

pub type SocketId = u64;
type SubscriptionId = u64;

#[derive(Debug)]
pub struct NewHeadsSubscription {
    pub id: SubscriptionId,
}

#[derive(Debug)]
pub enum Subscription {
    NewHeads(NewHeadsSubscription),
    TransactionStatus,
    PendingTransactions,
    Events,
    Reorg,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewHeadsNotification {
    pub subscription_id: SubscriptionId,
    pub result: BlockHeader,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum SubscriptionResponse {
    NewHeadsConfirmation(SubscriptionId),
    TransactionStatusConfirmation,
    PendingTransactionsConfirmation,
    EventsConfirmation,
    UnsubscriptionConfirmation(bool),

    NewHeadsNotification(NewHeadsNotification),
    TransactionStatusNotification,
    PendingTransactionsNotification,
    EventsNotification,
}

pub struct SocketContext {
    /// The sender part of the socket's own channel
    pub(crate) starknet_sender: Sender<SubscriptionResponse>,
    pub(crate) subscriptions: Vec<Subscription>,
}

impl SocketContext {
    pub fn from_sender(sender: Sender<SubscriptionResponse>) -> Self {
        Self { starknet_sender: sender, subscriptions: vec![] }
    }
}
