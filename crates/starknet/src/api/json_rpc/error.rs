use serde::Serialize;
use server::rpc_core::{error::RpcError, response::ResponseResult};
use tracing::error;

use super::models::{transaction::TransactionHashHex, BlockId};

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Rpc error {0:?}")]
    RpcError(RpcError),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Contract not found")]
    ContractNotFound,
    #[error("Transaction {0} not found")]
    TransactionNotFound(TransactionHashHex),
    #[error("Transaction idx {0:?} not found in block {1:?}")]
    InvalidTransactionIndexInBlock(usize, BlockId),
}

pub(crate) type RpcResult<T> = std::result::Result<T, ApiError>;

/// Helper trait to easily convert results to rpc results
pub(crate) trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

/// Converts a serializable value into a `ResponseResult`
pub fn to_rpc_result<T: Serialize>(val: T) -> ResponseResult {
    match serde_json::to_value(val) {
        Ok(success) => ResponseResult::Success(success),
        Err(err) => {
            error!("Failed serialize rpc response: {:?}", err);
            ResponseResult::error(RpcError::internal_error())
        }
    }
}

impl<T: Serialize> ToRpcResponseResult for RpcResult<T> {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(data) => to_rpc_result(data),
            Err(err) => match err {
                ApiError::RpcError(rpc_error) => rpc_error,
                err @ ApiError::BlockNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(24),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ContractNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(20),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::TransactionNotFound(_) => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(25),
                    message: err.to_string().into(),
                    data: None,
                },
                ApiError::InvalidTransactionIndexInBlock(_, _) => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(27),
                    message: err.to_string().into(),
                    data: None,
                },
            }
            .into(),
        }
    }
}
