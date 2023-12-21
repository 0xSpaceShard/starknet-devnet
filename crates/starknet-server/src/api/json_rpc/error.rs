use serde_json::json;
use server::rpc_core::error::RpcError;
use starknet_types;
use thiserror::Error;
use tracing::error;

use super::{StarknetResponse, WILDCARD_RPC_ERROR_CODE};

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
    ContractError { error: starknet_core::error::Error },
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
    #[error("Max fee is smaller than the minimal transaction cost (validation plus fee transfer)")]
    InsufficientMaxFee,
    #[error("Account balance is smaller than the transaction's max_fee")]
    InsufficientAccountBalance,
    #[error("Account validation failed")]
    ValidationFailure { reason: String },
    #[error("No trace available for transaction")]
    NoTraceAvailable,
}

impl ApiError {
    pub(crate) fn api_error_to_rpc_error(self) -> RpcError {
        let error_message = self.to_string();
        match self {
            ApiError::RpcError(rpc_error) => rpc_error,
            ApiError::BlockNotFound => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(24),
                message: error_message.into(),
                data: None,
            },
            ApiError::ContractNotFound => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(20),
                message: error_message.into(),
                data: None,
            },
            ApiError::TransactionNotFound => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(29),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidTransactionIndexInBlock => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(27),
                message: error_message.into(),
                data: None,
            },
            ApiError::ClassHashNotFound => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(28),
                message: error_message.into(),
                data: None,
            },
            ApiError::ContractError { error: inner_error } => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(40),
                message: error_message.into(),
                data: Some(json!(
                    {
                        "revert_error": anyhow::format_err!(inner_error).root_cause().to_string()
                    }
                )),
            },
            ApiError::NoBlocks => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(32),
                message: error_message.into(),
                data: None,
            },
            ApiError::RequestPageSizeTooBig => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(31),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidContinuationToken => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(33),
                message: error_message.into(),
                data: None,
            },
            ApiError::TooManyKeysInFilter => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(34),
                message: error_message.into(),
                data: None,
            },
            ApiError::ClassAlreadyDeclared => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(51),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidContractClass => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(50),
                message: error_message.into(),
                data: None,
            },
            ApiError::TypesError(_) => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: error_message.into(),
                data: None,
            },
            ApiError::OnlyLatestBlock => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(24),
                message: error_message.into(),
                data: None,
            },
            ApiError::UnsupportedAction { msg } => RpcError {
                code: server::rpc_core::error::ErrorCode::InvalidRequest,
                message: msg.into(),
                data: None,
            },
            ApiError::InsufficientMaxFee => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(53),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidTransactionNonce => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(52),
                message: error_message.into(),
                data: None,
            },
            ApiError::InsufficientAccountBalance => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(54),
                message: error_message.into(),
                data: None,
            },
            ApiError::ValidationFailure { reason } => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(55),
                message: error_message.into(),
                data: Some(serde_json::Value::String(reason)),
            },
            ApiError::StarknetDevnetError(
                starknet_core::error::Error::TransactionValidationError(validation_error),
            ) => {
                let api_err = match validation_error {
                    starknet_core::error::TransactionValidationError::InsufficientMaxFee => ApiError::InsufficientMaxFee,
                    starknet_core::error::TransactionValidationError::InvalidTransactionNonce => ApiError::InvalidTransactionNonce,
                    starknet_core::error::TransactionValidationError::InsufficientAccountBalance => ApiError::InsufficientAccountBalance,
                    starknet_core::error::TransactionValidationError::ValidationFailure { reason } => ApiError::ValidationFailure { reason },
                };

                api_err.api_error_to_rpc_error()
            }
            ApiError::StarknetDevnetError(error) => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: anyhow::format_err!(error).root_cause().to_string().into(),
                data: None,
            },
            ApiError::NoTraceAvailable => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(10),
                message: error_message.into(),
                data: None,
            },
        }
    }
}

pub(crate) type StrictRpcResult = Result<StarknetResponse, ApiError>;

#[cfg(test)]
mod tests {
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
        fn test_error() -> starknet_core::error::Error {
            starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::ValidationFailure {
                    reason: "some reason".into(),
                },
            )
        }
        let error_expected_message = anyhow::format_err!(test_error()).root_cause().to_string();

        error_expected_code_and_message(
            ApiError::ContractError { error: test_error() },
            40,
            "Contract error",
        );

        // check contract error data property
        let error = ApiError::ContractError { error: test_error() }.api_error_to_rpc_error();

        assert_eq!(
            error.data.unwrap().get("revert_error").unwrap().as_str().unwrap(),
            &error_expected_message
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
                starknet_core::error::TransactionValidationError::InsufficientMaxFee,
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::InsufficientMaxFee.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::InsufficientMaxFee,
            53,
            "Max fee is smaller than the minimal transaction cost (validation plus fee transfer)",
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
            &reason,
        );
    }

    fn error_expected_code_and_message(err: ApiError, expected_code: i64, expected_message: &str) {
        let error_result = StrictRpcResult::Err(err).to_rpc_result();
        match error_result {
            server::rpc_core::response::ResponseResult::Success(_) => panic!("Expected error"),
            server::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(err.message, expected_message);
                assert_eq!(err.code, server::rpc_core::error::ErrorCode::ServerError(expected_code))
            }
        }
    }

    fn error_expected_code_and_data(err: ApiError, expected_code: i64, expected_data: &str) {
        let error_result = StrictRpcResult::Err(err).to_rpc_result();
        match error_result {
            server::rpc_core::response::ResponseResult::Success(_) => panic!("Expected error"),
            server::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(err.data.unwrap().as_str().unwrap(), expected_data);
                assert_eq!(err.code, server::rpc_core::error::ErrorCode::ServerError(expected_code))
            }
        }
    }
}
