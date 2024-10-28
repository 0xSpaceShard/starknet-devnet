use serde::{self, Serialize};
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

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewHeadsNotification {
    pub subscription_id: SubscriptionId,
    pub result: BlockHeader,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub enum SubscriptionConfirmation {
    NewHeadsConfirmation(SubscriptionId),
    TransactionStatusConfirmation,
    PendingTransactionsConfirmation,
    EventsConfirmation,
    UnsubscriptionConfirmation(bool),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum SubscriptionNotification {
    #[serde(rename = "starknet_subscriptionNewHeads")]
    NewHeadsNotification(NewHeadsNotification),
    #[serde(rename = "starknet_subscriptionTransactionStatus")]
    TransactionStatusNotification,
    #[serde(rename = "starknet_subscriptionPendingTransactions")]
    PendingTransactionsNotification,
    #[serde(rename = "starknet_subscriptionEvents")]
    EventsNotification,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SubscriptionResponse {
    Confirmation(SubscriptionConfirmation),
    Notification(SubscriptionNotification),
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
