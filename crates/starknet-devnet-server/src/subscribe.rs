use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use serde::{self, Serialize};
use starknet_types::rpc::block::BlockHeader;
use tokio::sync::Mutex;

use crate::rpc_core::request::Id;

pub type SocketId = u64;

#[derive(Debug)]
pub enum Subscription {
    NewHeads(Id),
    TransactionStatus,
    PendingTransactions,
    Events,
    Reorg,
}

type SubscriptionId = Id;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SubscriptionConfirmation {
    NewHeadsConfirmation(SubscriptionId),
    TransactionStatusConfirmation(SubscriptionId),
    PendingTransactionsConfirmation(SubscriptionId),
    EventsConfirmation(SubscriptionId),
    UnsubscriptionConfirmation(bool),
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SubscriptionNotification {
    NewHeadsNotification(BlockHeader),
    TransactionStatusNotification,
    PendingTransactionsNotification,
    EventsNotification,
}

impl SubscriptionNotification {
    fn method_name(&self) -> &'static str {
        match self {
            SubscriptionNotification::NewHeadsNotification(_) => "starknet_subscriptionNewHeads",
            SubscriptionNotification::TransactionStatusNotification => {
                "starknet_subscriptionTransactionStatus"
            }
            SubscriptionNotification::PendingTransactionsNotification => {
                "starknet_subscriptionPendingTransactions"
            }
            SubscriptionNotification::EventsNotification => "starknet_subscriptionEvents",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SubscriptionResponse {
    Confirmation { rpc_request_id: Id, result: SubscriptionConfirmation },
    Notification { subscription_id: Id, data: SubscriptionNotification },
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
    subscriptions: Vec<Subscription>,
}

impl SocketContext {
    pub fn from_sender(sender: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Self {
        Self { sender, subscriptions: vec![] }
    }

    async fn send(&self, subscription_response: SubscriptionResponse) {
        let resp_serialized = subscription_response.to_serialized_rpc_response().to_string();

        if let Err(e) = self.sender.lock().await.send(Message::Text(resp_serialized)).await {
            tracing::error!("Failed writing to socket: {}", e.to_string());
        }
    }

    pub async fn subscribe(&mut self, rpc_request_id: Id) -> SubscriptionId {
        let subscription_id = Id::Number(rand::random()); // TODO safe? negative?
        self.subscriptions.push(Subscription::NewHeads(subscription_id.clone()));

        self.send(SubscriptionResponse::Confirmation {
            rpc_request_id,
            result: SubscriptionConfirmation::NewHeadsConfirmation(subscription_id.clone()),
        })
        .await;

        subscription_id
    }

    pub async fn notify(&self, subscription_id: SubscriptionId, data: SubscriptionNotification) {
        self.send(SubscriptionResponse::Notification { subscription_id, data }).await;
    }

    pub async fn notify_subscribers(&self, data: SubscriptionNotification) {
        for subscription in self.subscriptions.iter() {
            match subscription {
                Subscription::NewHeads(subscription_id) => {
                    if let SubscriptionNotification::NewHeadsNotification(_) = data {
                        self.notify(subscription_id.clone(), data.clone()).await;
                    }
                }
                other => println!("DEBUG unsupported subscription: {other:?}"),
            }
        }
    }
}
