use starknet_core::stack_trace::{ErrorStack, Frame};
use starknet_types;
use thiserror::Error;
use tracing::error;

use super::{JsonRpcResponse, WILDCARD_RPC_ERROR_CODE};
use crate::api::http::error::HttpApiError;
use crate::rpc_core::error::RpcError;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    StarknetDevnetError(#[from] starknet_core::error::Error),
    #[error("Types error")]
    TypesError(#[from] starknet_types::error::Error),
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
    ContractError { error_stack: ErrorStack },
    #[error("Transaction execution error")]
    TransactionExecutionError { failure_index: usize, error_stack: ErrorStack },
    #[error("There are no blocks")]
    NoBlocks,
    #[error("Requested page size is too big")]
    RequestPageSizeTooBig,
    #[error("The supplied continuation token is invalid or unknown")]
    InvalidContinuationToken,
    #[error("Too many keys provided in a filter")]
    TooManyKeysInFilter,
    #[error("Class already declared")]
    ClassAlreadyDeclared,
    #[error("Invalid contract class")]
    InvalidContractClass,
    #[error("Only latest/pending block is supported")]
    OnlyLatestBlock,
    #[error("{msg}")]
    UnsupportedAction { msg: String },
    #[error("Invalid transaction nonce")]
    InvalidTransactionNonce,
    #[error("The transaction's resources don't cover validation or the minimal transaction fee")]
    InsufficientResourcesForValidate,
    #[error("Account balance is smaller than the transaction's max_fee")]
    InsufficientAccountBalance,
    #[error("Account validation failed")]
    ValidationFailure { reason: String },
    #[error("No trace available for transaction")]
    NoTraceAvailable,
    #[error("{msg}")]
    NoStateAtBlock { msg: String },
    #[error(transparent)]
    HttpApiError(#[from] HttpApiError),
    #[error("the compiled class hash did not match the one supplied in the transaction")]
    CompiledClassHashMismatch,
    #[error("Cannot go back more than 1024 blocks")]
    TooManyBlocksBack,
    #[error("This method does not support being called on the pending block")]
    CallOnPending,
    #[error("Invalid subscription id")]
    InvalidSubscriptionId,
}

impl ApiError {
    pub fn api_error_to_rpc_error(self) -> RpcError {
        let error_message = self.to_string();
        match self {
            ApiError::RpcError(rpc_error) => rpc_error,
            ApiError::BlockNotFound => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(24),
                message: error_message.into(),
                data: None,
            },
            ApiError::ContractNotFound => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(20),
                message: error_message.into(),
                data: None,
            },
            ApiError::TransactionNotFound => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(29),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidTransactionIndexInBlock => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(27),
                message: error_message.into(),
                data: None,
            },
            ApiError::ClassHashNotFound => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(28),
                message: error_message.into(),
                data: None,
            },
            ApiError::ContractError { error_stack } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(40),
                message: error_message.into(),
                data: Some(serialize_error_stack(&error_stack)),
            },
            ApiError::TransactionExecutionError { error_stack, failure_index } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(41),
                message: error_message.into(),
                data: Some(serde_json::json!({
                    "transaction_index": failure_index,
                    "execution_error": serialize_error_stack(&error_stack),
                })),
            },
            ApiError::NoBlocks => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(32),
                message: error_message.into(),
                data: None,
            },
            ApiError::RequestPageSizeTooBig => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(31),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidContinuationToken => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(33),
                message: error_message.into(),
                data: None,
            },
            ApiError::TooManyKeysInFilter => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(34),
                message: error_message.into(),
                data: None,
            },
            ApiError::ClassAlreadyDeclared => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(51),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidContractClass => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(50),
                message: error_message.into(),
                data: None,
            },
            ApiError::TypesError(_) => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: error_message.into(),
                data: None,
            },
            ApiError::OnlyLatestBlock => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(24),
                message: error_message.into(),
                data: None,
            },
            ApiError::UnsupportedAction { msg } => RpcError {
                code: crate::rpc_core::error::ErrorCode::InvalidRequest,
                message: msg.into(),
                data: None,
            },
            ApiError::InsufficientResourcesForValidate => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(53),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidTransactionNonce => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(52),
                message: error_message.into(),
                data: None,
            },
            ApiError::InsufficientAccountBalance => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(54),
                message: error_message.into(),
                data: None,
            },
            ApiError::ValidationFailure { reason } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(55),
                message: error_message.into(),
                data: Some(serde_json::Value::String(reason)),
            },
            ApiError::CompiledClassHashMismatch => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(60),
                message: error_message.into(),
                data: None,
            },
            ApiError::StarknetDevnetError(
                starknet_core::error::Error::TransactionValidationError(validation_error),
            ) => {
                let api_err = match validation_error {
                    starknet_core::error::TransactionValidationError::InsufficientResourcesForValidate => ApiError::InsufficientResourcesForValidate,
                    starknet_core::error::TransactionValidationError::InvalidTransactionNonce => ApiError::InvalidTransactionNonce,
                    starknet_core::error::TransactionValidationError::InsufficientAccountBalance => ApiError::InsufficientAccountBalance,
                    starknet_core::error::TransactionValidationError::ValidationFailure { reason } => ApiError::ValidationFailure { reason },
                };

                api_err.api_error_to_rpc_error()
            }
            ApiError::StarknetDevnetError(error) => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: anyhow::format_err!(error).root_cause().to_string().into(),
                data: None,
            },
            ApiError::NoTraceAvailable => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(10),
                message: error_message.into(),
                data: None,
            },
            ApiError::NoStateAtBlock { .. } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: error_message.into(),
                data: None,
            },
            ApiError::HttpApiError(http_api_error) => http_api_error.http_api_error_to_rpc_error(),
            ApiError::TooManyBlocksBack => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(68),
                message: error_message.into(),
                data: None,
            },
            ApiError::CallOnPending => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(69),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidSubscriptionId => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(66),
                message: error_message.into(),
                data: None,
            },
        }
    }

    pub(crate) fn is_forwardable_to_origin(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::BlockNotFound
            | Self::TransactionNotFound
            | Self::NoStateAtBlock { .. }
            | Self::ClassHashNotFound => true,
            Self::StarknetDevnetError(_)
            | Self::NoTraceAvailable
            | Self::TypesError(_)
            | Self::RpcError(_)
            | Self::ContractNotFound
            | Self::InvalidTransactionIndexInBlock
            | Self::ContractError { .. }
            | Self::NoBlocks
            | Self::RequestPageSizeTooBig
            | Self::InvalidContinuationToken
            | Self::TooManyKeysInFilter
            | Self::ClassAlreadyDeclared
            | Self::InvalidContractClass
            | Self::OnlyLatestBlock
            | Self::UnsupportedAction { .. }
            | Self::InvalidTransactionNonce
            | Self::InsufficientAccountBalance
            | Self::ValidationFailure { .. }
            | Self::HttpApiError(_)
            | Self::TransactionExecutionError { .. }
            | Self::CallOnPending
            | Self::TooManyBlocksBack
            | Self::InvalidSubscriptionId
            | Self::InsufficientResourcesForValidate
            | Self::CompiledClassHashMismatch => false,
        }
    }
}

/// Constructs a recursive object from the provided `error_stack`. The topmost call (the first in
/// the stack's vector) is the outermost in the returned object. Vm frames are skipped as they don't
/// support nesting (no recursive properties).
fn serialize_error_stack(error_stack: &ErrorStack) -> serde_json::Value {
    let mut recursive_error = serde_json::json!(null);

    for frame in error_stack.stack.iter().rev() {
        match frame {
            Frame::EntryPoint(entry_point_error_frame) => {
                recursive_error = serde_json::json!({
                    "contract_address": entry_point_error_frame.storage_address,
                    "class_hash": entry_point_error_frame.class_hash,
                    "selector": entry_point_error_frame.selector,
                    "error": recursive_error,
                });
            }
            Frame::Vm(_) => { /* do nothing */ }
            Frame::StringFrame(msg) => {
                recursive_error = serde_json::json!(*msg);
            }
        };
    }

    recursive_error
}

pub type StrictRpcResult = Result<JsonRpcResponse, ApiError>;

#[cfg(test)]
mod tests {
    use starknet_core::stack_trace::ErrorStack;

    use super::StrictRpcResult;
    use crate::api::json_rpc::error::ApiError;
    use crate::api::json_rpc::ToRpcResponseResult;

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
        error_expected_code_and_message(
            ApiError::TransactionNotFound,
            29,
            "Transaction hash not found",
        );
    }

    #[test]
    fn invalid_transaction_index_error() {
        error_expected_code_and_message(
            ApiError::InvalidTransactionIndexInBlock,
            27,
            "Invalid transaction index in a block",
        );
    }

    #[test]
    fn class_hash_not_found_error() {
        error_expected_code_and_message(ApiError::ClassHashNotFound, 28, "Class hash not found");
    }

    #[test]
    fn page_size_too_big_error() {
        error_expected_code_and_message(
            ApiError::RequestPageSizeTooBig,
            31,
            "Requested page size is too big",
        );
    }

    #[test]
    fn no_blocks_error() {
        error_expected_code_and_message(ApiError::NoBlocks, 32, "There are no blocks");
    }

    #[test]
    fn invalid_continuation_token_error() {
        error_expected_code_and_message(
            ApiError::InvalidContinuationToken,
            33,
            "The supplied continuation token is invalid or unknown",
        );
    }

    #[test]
    fn too_many_keys_in_filter_error() {
        error_expected_code_and_message(
            ApiError::TooManyKeysInFilter,
            34,
            "Too many keys provided in a filter",
        );
    }

    #[test]
    fn contract_error() {
        let api_error =
            ApiError::ContractError { error_stack: ErrorStack::from_str_err("some_reason") };

        error_expected_code_and_message(api_error, 40, "Contract error");

        // check contract error data property
        let error =
            ApiError::ContractError { error_stack: ErrorStack::from_str_err("some_reason") }
                .api_error_to_rpc_error();

        assert_eq!(error.data.unwrap().as_str().unwrap(), "some_reason");
    }

    #[test]
    fn transaction_execution_error() {
        error_expected_code_and_message(
            ApiError::TransactionExecutionError {
                failure_index: 0,
                error_stack: ErrorStack::from_str_err("anything"),
            },
            41,
            "Transaction execution error",
        );

        error_expected_code_and_data(
            ApiError::TransactionExecutionError {
                failure_index: 1,
                error_stack: ErrorStack::from_str_err("anything"),
            },
            41,
            &serde_json::json!({ "transaction_index": 1, "execution_error": "anything" }),
        );
    }

    #[test]
    fn invalid_transaction_nonce_error() {
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::InvalidTransactionNonce,
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::InvalidTransactionNonce.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::InvalidTransactionNonce,
            52,
            "Invalid transaction nonce",
        );
    }

    #[test]
    fn insufficient_max_fee_error() {
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::InsufficientResourcesForValidate,
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::InsufficientResourcesForValidate.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::InsufficientResourcesForValidate,
            53,
            "The transaction's resources don't cover validation or the minimal transaction fee",
        );
    }

    #[test]
    fn insufficient_account_balance_error() {
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::InsufficientAccountBalance,
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::InsufficientAccountBalance.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::InsufficientAccountBalance,
            54,
            "Account balance is smaller than the transaction's max_fee",
        );
    }

    #[test]
    fn account_validation_error() {
        let reason = String::from("some reason");
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::ValidationFailure {
                    reason: reason.clone(),
                },
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::ValidationFailure { reason: reason.clone() }.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::ValidationFailure { reason: reason.clone() },
            55,
            "Account validation failed",
        );

        error_expected_code_and_data(
            ApiError::ValidationFailure { reason: reason.clone() },
            55,
            &serde_json::json!(reason),
        );
    }

    fn error_expected_code_and_message(err: ApiError, expected_code: i64, expected_message: &str) {
        let error_result = StrictRpcResult::Err(err).to_rpc_result();
        match error_result {
            crate::rpc_core::response::ResponseResult::Success(_) => panic!("Expected error"),
            crate::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(err.message, expected_message);
                assert_eq!(err.code, crate::rpc_core::error::ErrorCode::ServerError(expected_code))
            }
        }
    }

    fn error_expected_code_and_data(
        err: ApiError,
        expected_code: i64,
        expected_data: &serde_json::Value,
    ) {
        let error_result = StrictRpcResult::Err(err).to_rpc_result();
        match error_result {
            crate::rpc_core::response::ResponseResult::Success(_) => panic!("Expected error"),
            crate::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(&err.data.unwrap(), expected_data);
                assert_eq!(err.code, crate::rpc_core::error::ErrorCode::ServerError(expected_code))
            }
        }
    }
}
