use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::SplitSink;
use serde::{self, Deserialize, Serialize};
use starknet_core::starknet::events::check_if_filter_applies_for_event;
use starknet_rust::core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::SubscriptionEmittedEvent;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::{BlockHeader, ReorgData};
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::{
    TransactionFinalityStatus, TransactionStatus, TransactionWithHash,
};
use tokio::sync::Mutex;

use crate::api::error::ApiError;
use crate::api::models::SubscriptionId;
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
        self.sockets
            .iter_mut()
            .for_each(|(_, socket_context)| socket_context.subscriptions.clear());
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
    status_container: Vec<TransactionFinalityStatus>,
}

impl StatusFilter {
    pub(crate) fn new(status_container: Vec<TransactionFinalityStatus>) -> Self {
        Self { status_container }
    }

    pub(crate) fn passes(&self, status: &TransactionFinalityStatus) -> bool {
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
    NewTransactionReceipts {
        address_filter: AddressFilter,
        status_filter: StatusFilter,
    },
    Events {
        address: Option<ContractAddress>,
        keys_filter: Option<Vec<Vec<Felt>>>,
        status_filter: StatusFilter,
    },
}

impl Subscription {
    fn confirm(&self, id: SubscriptionId) -> SubscriptionConfirmation {
        match self {
            Subscription::NewHeads => SubscriptionConfirmation::NewSubscription(id),
            Subscription::TransactionStatus { .. } => SubscriptionConfirmation::NewSubscription(id),
            Subscription::NewTransactions { .. } => SubscriptionConfirmation::NewSubscription(id),
            Subscription::NewTransactionReceipts { .. } => {
                SubscriptionConfirmation::NewSubscription(id)
            }
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
                Subscription::NewTransactionReceipts { address_filter, status_filter },
                NotificationData::NewTransactionReceipt(NewTransactionReceiptNotification {
                    tx_receipt,
                    sender_address,
                }),
            ) => {
                status_filter.passes(tx_receipt.finality_status())
                    && match sender_address {
                        Some(address) => address_filter.passes(address),
                        None => true,
                    }
            }
            (
                Subscription::Events { address, keys_filter, status_filter },
                NotificationData::Event(event_with_finality_status),
            ) => {
                let event = (&event_with_finality_status.emitted_event).into();
                check_if_filter_applies_for_event(address, keys_filter, &event)
                    && status_filter.passes(&event_with_finality_status.finality_status)
            }
            (
                Subscription::NewHeads
                | Subscription::TransactionStatus { .. }
                | Subscription::Events { .. }
                | Subscription::NewTransactions { .. }
                | Subscription::NewTransactionReceipts { .. },
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatusWithoutL1 {
    Received,
    Candidate,
    PreConfirmed,
    AcceptedOnL2,
}

impl From<TransactionFinalityStatusWithoutL1> for TransactionFinalityStatus {
    fn from(status: TransactionFinalityStatusWithoutL1) -> Self {
        match status {
            TransactionFinalityStatusWithoutL1::PreConfirmed => Self::PreConfirmed,
            TransactionFinalityStatusWithoutL1::AcceptedOnL2 => Self::AcceptedOnL2,
        }
    }
}

impl From<TransactionStatusWithoutL1> for TransactionFinalityStatus {
    fn from(status: TransactionStatusWithoutL1) -> Self {
        match status {
            TransactionStatusWithoutL1::Received => Self::Received,
            TransactionStatusWithoutL1::Candidate => Self::Candidate,
            TransactionStatusWithoutL1::PreConfirmed => Self::PreConfirmed,
            TransactionStatusWithoutL1::AcceptedOnL2 => Self::AcceptedOnL2,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct NewTransactionNotification {
    #[serde(flatten)]
    pub tx: TransactionWithHash,
    pub finality_status: TransactionFinalityStatus,
}

#[derive(Debug, Clone)]
pub struct NewTransactionReceiptNotification {
    pub tx_receipt: TransactionReceipt,
    pub sender_address: Option<ContractAddress>,
}

#[derive(Debug, Clone)]
pub enum NotificationData {
    NewHeads(BlockHeader),
    TransactionStatus(NewTransactionStatus),
    NewTransaction(NewTransactionNotification),
    NewTransactionReceipt(NewTransactionReceiptNotification),
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
    #[serde(rename = "starknet_subscriptionNewTransactionReceipts")]
    NewTransactionReceipt { subscription_id: SubscriptionId, result: TransactionReceipt },
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

    async fn send_serialized(&self, resp: String) {
        if let Err(e) = self.sender.lock().await.send(Message::Text(resp.into())).await {
            tracing::error!("Failed writing to socket: {}", e.to_string());
        }
    }

    pub async fn send_rpc_response(&self, result: serde_json::Value, id: Id) {
        let resp_serialized = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        })
        .to_string();

        tracing::trace!(target: "ws.json-rpc-api", response = %resp_serialized, "JSON-RPC response");
        self.send_serialized(resp_serialized).await;
    }

    async fn send_subscription_response(&self, resp: SubscriptionResponse) {
        let resp_serialized = resp.to_serialized_rpc_response().to_string();

        tracing::trace!(target: "ws.subscriptions", response = %resp_serialized, "subscription response");
        self.send_serialized(resp_serialized).await;
    }

    pub async fn subscribe(
        &mut self,
        rpc_request_id: Id,
        subscription: Subscription,
    ) -> SubscriptionId {
        loop {
            let subscription_id: SubscriptionId = rand::random::<u64>().into();
            if self.subscriptions.contains_key(&subscription_id) {
                continue;
            }

            let confirmation = subscription.confirm(subscription_id);
            self.subscriptions.insert(subscription_id, subscription);

            self.send_subscription_response(SubscriptionResponse::Confirmation {
                rpc_request_id,
                result: confirmation,
            })
            .await;

            return subscription_id;
        }
    }

    pub async fn unsubscribe(
        &mut self,
        rpc_request_id: Id,
        subscription_id: SubscriptionId,
    ) -> Result<(), ApiError> {
        self.subscriptions.remove(&subscription_id).ok_or(ApiError::InvalidSubscriptionId)?;
        self.send_subscription_response(SubscriptionResponse::Confirmation {
            rpc_request_id,
            result: SubscriptionConfirmation::Unsubscription(true),
        })
        .await;
        Ok(())
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

            NotificationData::NewTransaction(tx_notification) => {
                SubscriptionNotification::NewTransaction {
                    subscription_id,
                    result: tx_notification,
                }
            }

            NotificationData::NewTransactionReceipt(tx_receipt_notification) => {
                SubscriptionNotification::NewTransactionReceipt {
                    subscription_id,
                    result: tx_receipt_notification.tx_receipt,
                }
            }

            NotificationData::Event(emitted_event) => {
                SubscriptionNotification::Event { subscription_id, result: emitted_event }
            }

            NotificationData::Reorg(reorg_data) => {
                SubscriptionNotification::Reorg { subscription_id, result: reorg_data }
            }
        };

        self.send_subscription_response(SubscriptionResponse::Notification(Box::new(
            notification_data,
        )))
        .await;
    }

    pub async fn notify_subscribers(&self, notification: &NotificationData) {
        for (subscription_id, subscription) in self.subscriptions.iter() {
            if subscription.matches(notification) {
                self.notify(*subscription_id, notification.clone()).await;
            }
        }
    }
}
