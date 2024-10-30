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
#[serde(tag = "method", content = "params")]
pub enum SubscriptionNotification {
    #[serde(rename = "starknet_subscriptionNewHeads")]
    NewHeadsNotification(BlockHeader),
    #[serde(rename = "starknet_subscriptionTransactionStatus")]
    TransactionStatusNotification,
    #[serde(rename = "starknet_subscriptionPendingTransactions")]
    PendingTransactionsNotification,
    #[serde(rename = "starknet_subscriptionEvents")]
    EventsNotification,
}

#[derive(Debug, Clone)]
pub enum SubscriptionResponse {
    Confirmation { rpc_request_id: Id, result: SubscriptionConfirmation },
    Notification { subscription_id: Id, data: SubscriptionNotification },
}

impl SubscriptionResponse {
    fn to_serialized_rpc_response(&self) -> Result<serde_json::Value, serde_json::Error> {
        let mut resp = match self {
            SubscriptionResponse::Confirmation { rpc_request_id, result } => {
                serde_json::json!({
                    "id": rpc_request_id,
                    "result": result, // TODO nested or not?
                })
            }
            SubscriptionResponse::Notification { subscription_id, data } => {
                let mut resp = serde_json::to_value(data)?;
                resp["params"]["id"] = serde_json::to_value(subscription_id)?;
                resp
            }
        };

        resp["jsonrpc"] = "2.0".into();
        Ok(resp)
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
        let resp_serialized = match subscription_response.to_serialized_rpc_response() {
            Ok(resp_serialized) => resp_serialized,
            Err(e) => {
                tracing::error!("Cannot serialize response: {e:?}");
                return;
            }
        };

        if let Err(e) =
            self.sender.lock().await.send(Message::Text(resp_serialized.to_string())).await
        {
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
