use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use serde::{self, Deserialize, Serialize};
use starknet_core::starknet::events::check_if_filter_applies_for_event;
use starknet_rs_core::types::{BlockTag, Felt};
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::EmittedEvent;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::{BlockHeader, ReorgData};
use starknet_types::rpc::transactions::{TransactionStatus, TransactionWithHash};
use tokio::sync::Mutex;

use crate::api::json_rpc::error::ApiError;
use crate::rpc_core::request::Id;

pub type SocketId = u64;

type SubscriptionId = i64;

#[derive(Debug)]
pub struct AddressFilter {
    address_container: Vec<ContractAddress>,
}

impl AddressFilter {
    pub(crate) fn new(address_container: Vec<ContractAddress>) -> Self {
        Self { address_container }
    }
    pub(crate) fn passes(&self, address: &ContractAddress) -> bool {
        self.address_container.is_empty() || self.address_container.contains(address)
    }
}

#[derive(Debug)]
pub enum Subscription {
    NewHeads,
    TransactionStatus { tag: BlockTag, transaction_hash: TransactionHash },
    PendingTransactionsFull { address_filter: AddressFilter },
    PendingTransactionsHash { address_filter: AddressFilter },
    Events { address: Option<ContractAddress>, keys_filter: Option<Vec<Vec<Felt>>> },
}

impl Subscription {
    fn confirm(&self, id: SubscriptionId) -> SubscriptionConfirmation {
        match self {
            Subscription::NewHeads => SubscriptionConfirmation::NewSubscription(id),
            Subscription::TransactionStatus { .. } => SubscriptionConfirmation::NewSubscription(id),
            Subscription::PendingTransactionsFull { .. }
            | Subscription::PendingTransactionsHash { .. } => {
                SubscriptionConfirmation::NewSubscription(id)
            }
            Subscription::Events { .. } => SubscriptionConfirmation::NewSubscription(id),
        }
    }

    pub fn matches(&self, notification: &NotificationData) -> bool {
        match (self, notification) {
            (Subscription::NewHeads, NotificationData::NewHeads(_)) => true,
            (
                Subscription::TransactionStatus { tag, transaction_hash: subscription_hash },
                NotificationData::TransactionStatus(notification),
            ) => {
                tag == &notification.origin_tag
                    && subscription_hash == &notification.transaction_hash
            }
            (
                Subscription::PendingTransactionsFull { address_filter },
                NotificationData::PendingTransaction(PendingTransactionNotification::Full(tx)),
            ) => match tx.get_sender_address() {
                Some(address) => address_filter.passes(&address),
                None => true,
            },
            (
                Subscription::PendingTransactionsHash { address_filter },
                NotificationData::PendingTransaction(PendingTransactionNotification::Hash(
                    hash_wrapper,
                )),
            ) => match hash_wrapper.sender_address {
                Some(address) => address_filter.passes(&address),
                None => true,
            },
            (Subscription::Events { address, keys_filter }, NotificationData::Event(event)) => {
                check_if_filter_applies_for_event(address, keys_filter, &event.into())
            }
            (
                Subscription::NewHeads
                | Subscription::TransactionStatus { .. }
                | Subscription::Events { .. },
                NotificationData::Reorg(_),
            ) => true, // any subscription other than pending tx requires reorg notification
            _ => false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Deserialize))]
pub(crate) enum SubscriptionConfirmation {
    NewSubscription(SubscriptionId),
    Unsubscription(bool),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct NewTransactionStatus {
    pub transaction_hash: TransactionHash,
    pub status: TransactionStatus,
    /// which block this notification originates from: pending or latest
    #[serde(skip)]
    #[cfg_attr(test, serde(default = "crate::test_utils::origin_tag_default"))]
    pub origin_tag: BlockTag,
}

#[derive(Debug, Clone)]
pub struct TransactionHashWrapper {
    pub hash: TransactionHash,
    pub sender_address: Option<ContractAddress>,
}

impl Serialize for TransactionHashWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.hash.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TransactionHashWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hash = Felt::deserialize(deserializer)?;

        Ok(TransactionHashWrapper { hash, sender_address: None })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Deserialize))]
pub enum PendingTransactionNotification {
    Hash(TransactionHashWrapper),
    Full(Box<TransactionWithHash>),
}

#[derive(Debug, Clone)]
pub enum NotificationData {
    NewHeads(BlockHeader),
    TransactionStatus(NewTransactionStatus),
    PendingTransaction(PendingTransactionNotification),
    Event(EmittedEvent),
    Reorg(ReorgData),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Deserialize))]
pub(crate) enum SubscriptionResponse {
    Confirmation {
        #[serde(rename = "id")]
        rpc_request_id: Id,
        result: SubscriptionConfirmation,
    },
    Notification(Box<SubscriptionNotification>),
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(tag = "method", content = "params")]
pub(crate) enum SubscriptionNotification {
    #[serde(rename = "starknet_subscriptionNewHeads")]
    NewHeads { subscription_id: SubscriptionId, result: BlockHeader },
    #[serde(rename = "starknet_subscriptionTransactionStatus")]
    TransactionStatus { subscription_id: SubscriptionId, result: NewTransactionStatus },
    #[serde(rename = "starknet_subscriptionPendingTransactions")]
    PendingTransaction { subscription_id: SubscriptionId, result: PendingTransactionNotification },
    #[serde(rename = "starknet_subscriptionEvents")]
    Event { subscription_id: SubscriptionId, result: EmittedEvent },
    #[serde(rename = "starknet_subscriptionReorg")]
    Reorg { subscription_id: SubscriptionId, result: ReorgData },
}

impl SubscriptionResponse {
    fn to_serialized_rpc_response(&self) -> serde_json::Value {
        let mut resp = serde_json::json!(self);

        resp["jsonrpc"] = "2.0".into();
        resp
    }
}

pub struct SocketContext {
    /// The sender part of the socket's own channel
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    subscriptions: HashMap<SubscriptionId, Subscription>,
}

impl SocketContext {
    pub fn from_sender(sender: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        Self { sender, subscriptions: HashMap::new() }
    }

    async fn send(&self, subscription_response: SubscriptionResponse) {
        let resp_serialized = subscription_response.to_serialized_rpc_response().to_string();

        if let Err(e) = self.sender.lock().await.send(Message::Text(resp_serialized)).await {
            tracing::error!("Failed writing to socket: {}", e.to_string());
        }
    }

    pub async fn subscribe(
        &mut self,
        rpc_request_id: Id,
        subscription: Subscription,
    ) -> SubscriptionId {
        let subscription_id = rand::random();

        let confirmation = subscription.confirm(subscription_id);
        self.subscriptions.insert(subscription_id, subscription);

        self.send(SubscriptionResponse::Confirmation { rpc_request_id, result: confirmation })
            .await;

        subscription_id
    }

    pub async fn unsubscribe(
        &mut self,
        rpc_request_id: Id,
        subscription_id: SubscriptionId,
    ) -> Result<(), ApiError> {
        match self.subscriptions.remove(&subscription_id) {
            Some(_) => {
                self.send(SubscriptionResponse::Confirmation {
                    rpc_request_id,
                    result: SubscriptionConfirmation::Unsubscription(true),
                })
                .await;
                Ok(())
            }
            None => Err(ApiError::InvalidSubscriptionId),
        }
    }

    pub async fn notify(&self, subscription_id: SubscriptionId, data: NotificationData) {
        let notification_data = match data {
            NotificationData::NewHeads(block_header) => {
                SubscriptionNotification::NewHeads { subscription_id, result: block_header }
            }

            NotificationData::TransactionStatus(new_transaction_status) => {
                SubscriptionNotification::TransactionStatus {
                    subscription_id,
                    result: new_transaction_status,
                }
            }

            NotificationData::PendingTransaction(pending_transaction_notification) => {
                SubscriptionNotification::PendingTransaction {
                    subscription_id,
                    result: pending_transaction_notification,
                }
            }

            NotificationData::Event(emitted_event) => {
                SubscriptionNotification::Event { subscription_id, result: emitted_event }
            }

            NotificationData::Reorg(reorg_data) => {
                SubscriptionNotification::Reorg { subscription_id, result: reorg_data }
            }
        };

        self.send(SubscriptionResponse::Notification(Box::new(notification_data))).await;
    }

    pub async fn notify_subscribers(&self, notification: &NotificationData) {
        for (subscription_id, subscription) in self.subscriptions.iter() {
            if subscription.matches(notification) {
                self.notify(*subscription_id, notification.clone()).await;
            }
        }
    }
}
