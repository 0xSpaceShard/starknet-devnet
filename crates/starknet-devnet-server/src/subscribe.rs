use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use serde::{self, Serialize};
use starknet_core::starknet::events::check_if_filter_applies_for_event;
use starknet_rs_core::types::{BlockTag, Felt};
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::EmittedEvent;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::BlockHeader;
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

    pub fn matches(&self, notification: &SubscriptionNotification) -> bool {
        match self {
            Subscription::NewHeads => {
                if let SubscriptionNotification::NewHeads(_) = notification {
                    return true;
                }
            }
            Subscription::TransactionStatus { tag, transaction_hash: subscription_hash } => {
                if let SubscriptionNotification::TransactionStatus(notification) = notification {
                    return tag == &notification.origin_tag
                        && subscription_hash == &notification.transaction_hash;
                }
            }
            Subscription::PendingTransactionsFull { address_filter, .. } => {
                if let SubscriptionNotification::PendingTransaction(
                    PendingTransactionNotification::Full(tx),
                ) = notification
                {
                    return match tx.get_sender_address() {
                        Some(address) => address_filter.passes(&address),
                        None => true,
                    };
                }
            }
            Subscription::PendingTransactionsHash { address_filter } => {
                if let SubscriptionNotification::PendingTransaction(
                    PendingTransactionNotification::Hash(hash_wrapper),
                ) = notification
                {
                    return match hash_wrapper.sender_address {
                        Some(address) => address_filter.passes(&address),
                        None => true,
                    };
                }
            }
            Subscription::Events { address, keys_filter } => {
                if let SubscriptionNotification::Event(event) = notification {
                    check_if_filter_applies_for_event(address, keys_filter, &event.into());
                }
            }
        }

        false
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SubscriptionConfirmation {
    NewSubscription(SubscriptionId),
    Unsubscription(bool),
}

#[derive(Debug, Clone, Serialize)]
pub struct NewTransactionStatus {
    pub transaction_hash: TransactionHash,
    pub status: TransactionStatus,
    /// which block this notification originates from: pending or latest
    #[serde(skip)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PendingTransactionNotification {
    Hash(TransactionHashWrapper),
    Full(Box<TransactionWithHash>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SubscriptionNotification {
    NewHeads(Box<BlockHeader>),
    TransactionStatus(NewTransactionStatus),
    PendingTransaction(PendingTransactionNotification),
    Event(EmittedEvent),
}

impl SubscriptionNotification {
    fn method_name(&self) -> &'static str {
        match self {
            SubscriptionNotification::NewHeads(_) => "starknet_subscriptionNewHeads",
            SubscriptionNotification::TransactionStatus(_) => {
                "starknet_subscriptionTransactionStatus"
            }
            SubscriptionNotification::PendingTransaction(_) => {
                "starknet_subscriptionPendingTransactions"
            }
            SubscriptionNotification::Event(_) => "starknet_subscriptionEvents",
        }
    }
}

#[derive(Debug)]
enum SubscriptionResponse {
    Confirmation { rpc_request_id: Id, result: SubscriptionConfirmation },
    Notification { subscription_id: SubscriptionId, data: Box<SubscriptionNotification> },
}

impl SubscriptionResponse {
    fn to_serialized_rpc_response(&self) -> serde_json::Value {
        let mut resp = match self {
            SubscriptionResponse::Confirmation { rpc_request_id, result } => {
                serde_json::json!({
                    "id": rpc_request_id,
                    "result": result,
                })
            }
            SubscriptionResponse::Notification { subscription_id, data } => {
                serde_json::json!({
                    "method": data.method_name(),
                    "params": {
                        "subscription_id": subscription_id,
                        "result": data,
                    }
                })
            }
        };

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

    pub async fn notify(&self, subscription_id: SubscriptionId, data: SubscriptionNotification) {
        self.send(SubscriptionResponse::Notification { subscription_id, data: Box::new(data) })
            .await;
    }

    pub async fn notify_subscribers(&self, notification: &SubscriptionNotification) {
        for (subscription_id, subscription) in self.subscriptions.iter() {
            if subscription.matches(notification) {
                self.notify(*subscription_id, notification.clone()).await;
            }
        }
    }
}
