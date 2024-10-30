use serde::{self, Serialize};
use starknet_types::rpc::block::BlockHeader;
use tokio::sync::mpsc::Sender;

use starknet_core::error::Error as CoreError;

use crate::{api::json_rpc::error::ApiError, rpc_core::request::Id};

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
    pub fn to_serialized_rpc_response(&self) -> serde_json::Value {
        let mut resp = match self {
            SubscriptionResponse::Confirmation { rpc_request_id, result } => {
                serde_json::json!({
                    "id": rpc_request_id,
                    "result": result,
                })
            }
            SubscriptionResponse::Notification { subscription_id, data } => {
                let mut resp = serde_json::to_value(data).unwrap();
                resp["params"]["id"] = serde_json::to_value(subscription_id).unwrap();
                resp
            }
        };

        resp["jsonrpc"] = "2.0".into();

        return resp;
    }
}

pub struct SocketContext {
    /// The sender part of the socket's own channel
    starknet_sender: Sender<SubscriptionResponse>,
    pub(crate) subscriptions: Vec<Subscription>,
}

impl SocketContext {
    pub fn from_sender(sender: Sender<SubscriptionResponse>) -> Self {
        Self { starknet_sender: sender, subscriptions: vec![] }
    }

    pub async fn subscribe(&mut self, rpc_request_id: Id) -> Result<SubscriptionId, ApiError> {
        let subscription_id = Id::Number(rand::random()); // TODO safe? negative?
        self.subscriptions.push(Subscription::NewHeads(subscription_id.clone()));

        self.starknet_sender
            .send(SubscriptionResponse::Confirmation {
                rpc_request_id,
                result: SubscriptionConfirmation::NewHeadsConfirmation(subscription_id.clone()),
            })
            .await
            .map_err(|e| {
                ApiError::StarknetDevnetError(CoreError::UnexpectedInternalError {
                    msg: e.to_string(),
                })
            })?;

        Ok(subscription_id)
    }

    pub async fn notify(
        &self,
        subscription_id: SubscriptionId,
        data: SubscriptionNotification,
    ) -> Result<(), ApiError> {
        self.starknet_sender
            .send(SubscriptionResponse::Notification { subscription_id, data })
            .await
            .map_err(|e| {
                ApiError::StarknetDevnetError(CoreError::UnexpectedInternalError {
                    msg: e.to_string(),
                })
            })?;

        Ok(())
    }
}
