use std::sync::Arc;

use serde_json::json;

use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::{ResponseResult, RpcResponse};

/// Used for forwarding requests to origin in case of:
/// - BlockNotFound
/// - TransactionNotFound
/// - NoStateAtBlock
/// - ClassHashNotFound
///
/// Basic contract-wise interaction is handled by `BlockingOriginReader`
#[derive(Clone)]
pub struct OriginForwarder {
    reqwest_client: reqwest::Client,
    url: Arc<String>,
    block_number: u64,
}

#[derive(Debug, thiserror::Error)]
enum OriginInteractionError {}

impl OriginForwarder {
    pub fn new(url: String, block_number: u64) -> Self {
        Self { reqwest_client: reqwest::Client::new(), url: Arc::new(url), block_number }
    }

    /// In case block tag "pending" or "latest" is a part of the request, it is replaced with the
    /// numeric block id of the forked block. Both JSON-RPC 1 and 2 semantics is covered
    fn clone_call_with_origin_block_id(&self, rpc_call: &RpcMethodCall) -> RpcMethodCall {
        let mut new_rpc_call = rpc_call.clone();
        let origin_block_id = json!({ "block_number": self.block_number });

        match new_rpc_call.params {
            crate::rpc_core::request::RequestParams::None => (),
            crate::rpc_core::request::RequestParams::Array(ref mut params) => {
                for param in params.iter_mut() {
                    if let Some("latest" | "pending") = param.as_str() {
                        *param = origin_block_id;
                        break;
                    }
                }
            }
            crate::rpc_core::request::RequestParams::Object(ref mut params) => {
                if let Some(block_id) = params.get_mut("block_id") {
                    *block_id = origin_block_id;
                }
            }
        }
        new_rpc_call
    }

    async fn call_with_error_handling(
        &self,
        rpc_call: &RpcMethodCall,
    ) -> Result<ResponseResult, anyhow::Error> {
        let rpc_call = self.clone_call_with_origin_block_id(rpc_call);
        let origin_rpc_resp: RpcResponse = self
            .reqwest_client
            .post(self.url.to_string())
            .json(&rpc_call)
            .send()
            .await?
            .json()
            .await?;

        Ok(origin_rpc_resp.result)
    }

    pub async fn call(&self, rpc_call: &RpcMethodCall) -> ResponseResult {
        match self.call_with_error_handling(rpc_call).await {
            Ok(result) => result,
            Err(e) => ResponseResult::Error(RpcError::internal_error_with::<String>(format!(
                "Error in interacting with origin: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OriginForwarder;
    use crate::rpc_core::request::RpcMethodCall;

    #[test]
    fn test_replacing_block_id() {
        let block_number = 10;
        let forwarder = OriginForwarder::new("http://dummy.com".to_string(), block_number);

        let common_body = json!({
            "method": "starknet_dummyMethod",
            "id": 1,
        });
        for (jsonrpc_value, orig_params, replaced_params) in [
            ("2.0", json!(null), json!(null)),
            ("1.0", json!(["a", 1, "latest", 2]), json!(["a", 1, { "block_number": 10 }, 2])),
            ("1.0", json!(["a", 1, "pending", 2]), json!(["a", 1, { "block_number": 10 }, 2])),
            (
                "2.0",
                json!({ "param1": "a", "param2": 1, "block_id": "latest", "param3": 2 }),
                json!({ "param1": "a", "param2": 1, "block_id": { "block_number": 10 }, "param3": 2 }),
            ),
            (
                "2.0",
                json!({ "param1": "a", "param2": 1, "block_id": "pending", "param3": 2 }),
                json!({ "param1": "a", "param2": 1, "block_id": { "block_number": 10 }, "param3": 2 }),
            ),
        ] {
            let mut orig_body = common_body.clone();
            orig_body["jsonrpc"] = serde_json::Value::String(jsonrpc_value.into());
            orig_body["params"] = orig_params;

            let request: RpcMethodCall = serde_json::from_value(orig_body).unwrap();
            let replaced_request = forwarder.clone_call_with_origin_block_id(&request);
            let replaced_request_json = serde_json::to_value(replaced_request).unwrap();

            let mut expected_body = common_body.clone();
            expected_body["jsonrpc"] = serde_json::Value::String(jsonrpc_value.into());
            expected_body["params"] = replaced_params;

            assert_eq!(replaced_request_json, expected_body);
        }
    }
}
