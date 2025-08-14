use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::SplitSink;
use serde::{self, Deserialize, Serialize};
use starknet_core::starknet::events::check_if_filter_applies_for_event;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::{SubscribableEventStatus, SubscriptionEmittedEvent};
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::{BlockHeader, ReorgData};
use starknet_types::rpc::transactions::{TransactionStatus, TransactionWithHash};
use tokio::sync::Mutex;

use crate::api::json_rpc::error::ApiError;
use crate::api::json_rpc::models::SubscriptionId;
use crate::rpc_core::request::Id;

pub type SocketId = u64;

#[derive(Default)]
pub struct SocketCollection {
    sockets: HashMap<SocketId, SocketContext>,
}

impl SocketCollection {
    pub fn get_mut(&mut self, socket_id: &SocketId) -> Result<&mut SocketContext, ApiError> {
        self.sockets.get_mut(socket_id).ok_or(ApiError::StarknetDevnetError(
            starknet_core::error::Error::UnexpectedInternalError {
                msg: format!("Unregistered socket ID: {socket_id}"),
            },
        ))
    }

    /// Assigns a random socket ID to the socket whose `socket_writer` is provided. Returns the ID.
    pub fn insert(&mut self, socket_writer: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> SocketId {
        let socket_id = rand::random();
        self.sockets.insert(socket_id, SocketContext::from_sender(socket_writer));
        socket_id
    }

    pub fn remove(&mut self, socket_id: &SocketId) {
        self.sockets.remove(socket_id);
    }

    pub async fn notify_subscribers(&self, notifications: &[NotificationData]) {
        for (_, socket_context) in self.sockets.iter() {
            for notification in notifications {
                socket_context.notify_subscribers(notification).await;
            }
        }
    }

    pub fn clear(&mut self) {
        self.sockets.clear();
        tracing::info!("Websocket memory cleared. No subscribers.");
    }
}

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

#[derive(Debug, Clone)]
pub struct StatusFilter {
    status_container: Vec<TransactionFinalityStatusWithoutL1>,
}

impl StatusFilter {
    pub(crate) fn new(status_container: Vec<TransactionFinalityStatusWithoutL1>) -> Self {
        Self { status_container }
    }
    pub(crate) fn passes(&self, status: &TransactionFinalityStatusWithoutL1) -> bool {
        self.status_container.is_empty() || self.status_container.contains(status)
    }
}

#[derive(Debug)]
pub enum Subscription {
    NewHeads,
    TransactionStatus {
        transaction_hash: TransactionHash,
    },
    NewTransactions {
        address_filter: AddressFilter,
        status_filter: StatusFilter,
    },
    Events {
        address: Option<ContractAddress>,
        keys_filter: Option<Vec<Vec<Felt>>>,
        finality_status_filter: SubscribableEventStatus,
    },
}

impl Subscription {
    fn confirm(&self, id: SubscriptionId) -> SubscriptionConfirmation {
        match self {
            Subscription::NewHeads => SubscriptionConfirmation::NewSubscription(id),
            Subscription::TransactionStatus { .. } => SubscriptionConfirmation::NewSubscription(id),
            Subscription::NewTransactions { .. } => SubscriptionConfirmation::NewSubscription(id),
            Subscription::Events { .. } => SubscriptionConfirmation::NewSubscription(id),
        }
    }

    pub fn matches(&self, notification: &NotificationData) -> bool {
        match (self, notification) {
            (Subscription::NewHeads, NotificationData::NewHeads(_)) => true,
            (
                Subscription::TransactionStatus { transaction_hash: subscription_hash },
                NotificationData::TransactionStatus(notification),
            ) => subscription_hash == &notification.transaction_hash,
            (
                Subscription::NewTransactions { address_filter, status_filter },
                NotificationData::NewTransaction(NewTransactionNotification {
                    tx,
                    finality_status,
                }),
            ) => match tx.get_sender_address() {
                Some(address) => {
                    address_filter.passes(&address) && status_filter.passes(finality_status)
                }
                None => true,
            },
            (
                Subscription::Events { address, keys_filter, finality_status_filter },
                NotificationData::Event(event_with_finality_status),
            ) => {
                let event = (&event_with_finality_status.emitted_event).into();
                check_if_filter_applies_for_event(address, keys_filter, &event)
                    && event_with_finality_status.finality_status == *finality_status_filter
            }
            (
                Subscription::NewHeads
                | Subscription::TransactionStatus { .. }
                | Subscription::Events { .. }
                | Subscription::NewTransactions { .. },
                NotificationData::Reorg(_),
            ) => true, // All subscriptions require a reorg notification
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionFinalityStatusWithoutL1 {
    PreConfirmed,
    AcceptedOnL2,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct NewTransactionNotification {
    #[serde(flatten)]
    pub tx: TransactionWithHash,
    pub finality_status: TransactionFinalityStatusWithoutL1,
}

#[derive(Debug, Clone)]
pub enum NotificationData {
    NewHeads(BlockHeader),
    TransactionStatus(NewTransactionStatus),
    NewTransaction(NewTransactionNotification),
    Event(SubscriptionEmittedEvent),
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
    #[serde(rename = "starknet_subscriptionNewTransaction")]
    NewTransaction { subscription_id: SubscriptionId, result: NewTransactionNotification },
    #[serde(rename = "starknet_subscriptionEvents")]
    Event { subscription_id: SubscriptionId, result: SubscriptionEmittedEvent },
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
        let subscription_id: SubscriptionId = rand::random::<u64>().into();

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

            NotificationData::NewTransaction(pending_transaction_notification) => {
                SubscriptionNotification::NewTransaction {
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
