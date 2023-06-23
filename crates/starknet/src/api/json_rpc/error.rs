use serde::Serialize;
use server::rpc_core::{error::RpcError, response::ResponseResult};

use tracing::error;



#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Rpc error {0:?}")]
    RpcError(RpcError),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Contract not found")]
    ContractNotFound,
    #[error("Transaction hash not found")]
    TransactionNotFound,
    #[error("Invalid transaction index in a block")]
    InvalidTransactionIndexInBlock,
    #[error("Class hash not found")]
    ClassHashNotFound,
    #[error("Contract error")]
    ContractError,
    #[error("There are no blocks")]
    NoBlocks,
    #[error("Requested page size is too big")]
    RequestPageSizeTooBig,
    #[error("The supplied continuation token is invalid or unknown")]
    InvalidContinuationToken,
    #[error("Too many keys provided in a filter")]
    TooManyKeysInFilter,
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
                err @ ApiError::TransactionNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(25),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::InvalidTransactionIndexInBlock => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(27),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ClassHashNotFound => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(28),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::ContractError => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(40),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::NoBlocks => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(32),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::RequestPageSizeTooBig => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(31),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::InvalidContinuationToken => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(33),
                    message: err.to_string().into(),
                    data: None,
                },
                err @ ApiError::TooManyKeysInFilter => RpcError {
                    code: server::rpc_core::error::ErrorCode::ServerError(34),
                    message: err.to_string().into(),
                    data: None,
                },
            }
            .into(),
        }
    }
}


#[cfg(test)]
mod tests{

    use crate::api::{json_rpc::error::{ApiError, RpcResult, ToRpcResponseResult}};

    #[test]
    fn contract_not_found_error() {
        error_expected_code_and_message(ApiError::ContractNotFound, 20, "Contract not found");
    }

    #[test]
    fn block_not_found_error() {
        error_expected_code_and_message(ApiError::BlockNotFound, 24, "Block not found");
    }

    #[test]
    fn transaction_not_found_error() {
        error_expected_code_and_message(ApiError::TransactionNotFound, 25, "Transaction hash not found");
    }

    #[test]
    fn invalid_transaction_index_error() {
        error_expected_code_and_message(ApiError::InvalidTransactionIndexInBlock, 27, "Invalid transaction index in a block");
    }

    #[test]
    fn class_hash_not_found_error() {
        error_expected_code_and_message(ApiError::ClassHashNotFound, 28, "Class hash not found");
    }

    #[test]
    fn page_size_too_big_error() {
        error_expected_code_and_message(ApiError::RequestPageSizeTooBig, 31, "Requested page size is too big");
    }

    #[test]
    fn no_blocks_error() {
        error_expected_code_and_message(ApiError::NoBlocks, 32, "There are no blocks");
    }

    #[test]
    fn invalid_continuation_token_error() {
        error_expected_code_and_message(ApiError::InvalidContinuationToken, 33, "The supplied continuation token is invalid or unknown");
    }

    #[test]
    fn too_many_keys_in_filter_error() {
        error_expected_code_and_message(ApiError::TooManyKeysInFilter, 34, "Too many keys provided in a filter");
    }

    #[test]
    fn contract_error() {
        error_expected_code_and_message(ApiError::ContractError, 40, "Contract error");
    }

    fn error_expected_code_and_message(err: ApiError, expected_code: i64, expected_message: &str) {
        let error_result = RpcResult::<()>::Err(err).to_rpc_result();
        match error_result  {
            server::rpc_core::response::ResponseResult::Success(_) => assert!(false),
            server::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(err.message, expected_message);
                assert_eq!(err.code, server::rpc_core::error::ErrorCode::ServerError(expected_code))
            },
        }
    }
}