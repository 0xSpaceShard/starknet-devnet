use std::sync::Arc;

use serde_json::json;
use starknet_rs_core::types::{
    BlockId as ImportedBlockId, BlockTag as ImportedBlockTag, BlockWithTxHashes,
    MaybePreConfirmedBlockWithTxHashes,
};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_types::rpc::block::{BlockId, BlockTag};

use super::error::ApiError;
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
    pub(crate) starknet_client: JsonRpcClient<HttpTransport>,
}

impl OriginForwarder {
    pub fn new(url: url::Url, block_number: u64) -> Self {
        Self {
            reqwest_client: reqwest::Client::new(),
            url: Arc::new(url.to_string()),
            block_number,
            starknet_client: JsonRpcClient::new(HttpTransport::new(url)),
        }
    }

    pub fn fork_block_number(&self) -> u64 {
        self.block_number
    }

    /// In case block tag "pre_confirmed" or "latest" is a part of the request, it is replaced with
    /// the numeric block id of the forked block. Both JSON-RPC 1 and 2 semantics is covered
    fn clone_call_with_origin_block_id(&self, rpc_call: &RpcMethodCall) -> RpcMethodCall {
        let mut new_rpc_call = rpc_call.clone();
        let origin_block_id = json!({ "block_number": self.block_number });

        match new_rpc_call.params {
            crate::rpc_core::request::RequestParams::None => (),
            crate::rpc_core::request::RequestParams::Array(ref mut params) => {
                for param in params.iter_mut() {
                    match param.as_str() {
                        Some("latest" | "pre_confirmed") => {
                            *param = origin_block_id;
                            break;
                        }
                        Some("l1_accepted") => {
                            tracing::warn!("Assuming fork block is ACCEPTED_ON_L1");
                            *param = origin_block_id;
                            break;
                        }
                        _ => (),
                    }
                }
            }
            crate::rpc_core::request::RequestParams::Object(ref mut params) => {
                if let Some(block_id) = params.get_mut("block_id") {
                    match block_id.as_str() {
                        Some("latest" | "pre_confirmed") => {
                            *block_id = origin_block_id;
                        }
                        Some("l1_accepted") => {
                            tracing::warn!("Assuming fork block is ACCEPTED_ON_L1");
                            *block_id = origin_block_id;
                        }
                        _ => (),
                    }
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

    async fn get_l1_accepted_block(&self) -> Result<BlockWithTxHashes, ApiError> {
        let tag = ImportedBlockId::Tag(ImportedBlockTag::L1Accepted);
        match self.starknet_client.get_block_with_tx_hashes(tag).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(block)) => Ok(block),
            other => Err(ApiError::StarknetDevnetError(
                starknet_core::error::Error::UnexpectedInternalError {
                    msg: format!(
                        "Failed retrieval of l1_accepted block from forking origin. Got: {other:?}"
                    ),
                },
            )),
        }
    }

    /// Only use with confirmed block ID
    pub(crate) async fn get_block_number_from_block_id(
        &self,
        block_id: BlockId,
    ) -> Result<u64, ApiError> {
        if block_id == BlockId::Tag(BlockTag::L1Accepted) {
            let l1_accepted_block = self.get_l1_accepted_block().await?;
            return Ok(std::cmp::min(l1_accepted_block.block_number, self.fork_block_number()));
        }

        match self.starknet_client.get_block_with_tx_hashes(ImportedBlockId::from(block_id)).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(block)) => Ok(block.block_number),
            Ok(MaybePreConfirmedBlockWithTxHashes::PreConfirmedBlock(block)) => {
                Err(ApiError::StarknetDevnetError(
                    starknet_core::error::Error::UnexpectedInternalError {
                        msg: format!("Impossible: expected a confirmed block; got: {block:?}"),
                    },
                ))
            }
            Err(error) => Err(ApiError::StarknetDevnetError(
                starknet_core::error::Error::UnexpectedInternalError {
                    msg: format!("Invalid origin response in retrieving block number: {error}"),
                },
            )),
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
        let forwarder =
            OriginForwarder::new(url::Url::parse("http://dummy.com").unwrap(), block_number);

        let common_body = json!({
            "method": "starknet_dummyMethod",
            "id": 1,
        });
        for (jsonrpc_value, orig_params, replaced_params) in [
            ("2.0", json!(null), json!(null)),
            ("1.0", json!(["a", 1, "latest", 2]), json!(["a", 1, { "block_number": 10 }, 2])),
            (
                "1.0",
                json!(["a", 1, "pre_confirmed", 2]),
                json!(["a", 1, { "block_number": 10 }, 2]),
            ),
            (
                "2.0",
                json!({ "param1": "a", "param2": 1, "block_id": "latest", "param3": 2 }),
                json!({ "param1": "a", "param2": 1, "block_id": { "block_number": 10 }, "param3": 2 }),
            ),
            (
                "2.0",
                json!({ "param1": "a", "param2": 1, "block_id": "pre_confirmed", "param3": 2 }),
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
