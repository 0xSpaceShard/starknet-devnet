use std::fmt::{self};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::ws::WebSocket;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{FutureExt, future};
use serde::de::DeserializeOwned;
use tracing::{trace, warn};

use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::{Request, RpcCall, RpcMethodCall};
use crate::rpc_core::response::{Response, ResponseResult, RpcResponse};

/// Helper trait that is used to execute starknet rpc calls
#[async_trait::async_trait]
pub trait RpcHandler: Clone + Send + Sync + 'static {
    /// The request type to expect
    type Request: DeserializeOwned + Send + Sync + fmt::Display;

    /// Invoked when the request was received
    async fn on_request(
        &self,
        request: Self::Request,
        original_call: RpcMethodCall,
    ) -> ResponseResult;

    /// Invoked for every incoming `RpcMethodCall`
    ///
    /// This will attempt to deserialize a `{ "method" : "<name>", "params": "<params>" }` message
    /// into the `Request` type of this handler. If a `Request` instance was deserialized
    /// successfully, [`Self::on_request`] will be invoked.
    ///
    /// **Note**: override this function if the expected `Request` deviates from `{ "method" :
    /// "<name>", "params": "<params>" }`
    async fn on_call(&self, call: RpcMethodCall) -> RpcResponse;

    /// Handles websocket connection, from start to finish.
    async fn on_websocket(&self, mut socket: WebSocket);
}

/// Handles incoming JSON-RPC Request
pub async fn handle<THandler: RpcHandler>(
    State(handler): State<THandler>,
    request: Result<Json<Request>, JsonRejection>,
) -> Json<Response> {
    match request {
        Ok(req) => handle_request(req.0, handler)
            .await
            .unwrap_or_else(|| Response::error(RpcError::invalid_request()))
            .into(),
        Err(err) => match &err {
            JsonRejection::JsonSyntaxError(e) => {
                let error_msg = e.to_string();
                warn!(target: "rpc", "JSON syntax error: {}", error_msg);
                Response::error(RpcError::parse_error(error_msg)).into()
            }
            JsonRejection::JsonDataError(e) => {
                warn!(target: "rpc", "JSON data error: {}", e);
                Response::error(RpcError::invalid_request_with_reason(format!("Data error: {}", e)))
                    .into()
            }
            JsonRejection::MissingJsonContentType(e) => {
                warn!(target: "rpc", "Missing JSON content type: {}", e);
                Response::error(RpcError::invalid_request_with_reason("Missing content type"))
                    .into()
            }
            _ => {
                warn!(target: "rpc", "Request rejection: {}", err);
                Response::error(RpcError::invalid_request_with_reason(err.to_string())).into()
            }
        },
    }
}

pub async fn handle_socket<THandler: RpcHandler>(
    ws_upgrade: WebSocketUpgrade,
    State(handler): State<THandler>,
) -> impl IntoResponse {
    tracing::info!("New websocket connection!");
    ws_upgrade.on_failed_upgrade(|e| tracing::error!("Failed websocket upgrade: {e:?}")).on_upgrade(
        move |socket| async move {
            handler.on_websocket(socket).await;
        },
    )
}

/// Handle the JSON-RPC [Request]
///
/// This will try to deserialize the payload into the request type of the handler and if successful
/// invoke the handler.
pub async fn handle_request<THandler: RpcHandler>(
    req: Request,
    handler: THandler,
) -> Option<Response> {
    /// processes batch calls
    fn responses_as_batch(outs: Vec<Option<RpcResponse>>) -> Option<Response> {
        let batch: Vec<_> = outs.into_iter().flatten().collect();
        (!batch.is_empty()).then_some(Response::Batch(batch))
    }

    match req {
        Request::Single(call) => handle_call(call, handler).await.map(Response::Single),
        Request::Batch(calls) => {
            future::join_all(calls.into_iter().map(move |call| handle_call(call, handler.clone())))
                .map(responses_as_batch)
                .await
        }
    }
}

/// handle a single RPC method call
pub(crate) async fn handle_call<THandler: RpcHandler>(
    call: RpcCall,
    handler: THandler,
) -> Option<RpcResponse> {
    match call {
        RpcCall::MethodCall(call) => {
            trace!(target: "rpc", id = ?call.id, method = ?call.method, "handling call");
            Some(handler.on_call(call).await)
        }
        RpcCall::Notification(notification) => {
            trace!(target: "rpc", method = ?notification.method, "received rpc notification");
            None
        }
        RpcCall::Invalid { id } => {
            warn!(target: "rpc", ?id, "invalid rpc call");
            Some(RpcResponse::invalid_request(id))
        }
    }
}
