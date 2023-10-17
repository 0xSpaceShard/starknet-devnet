use server::rpc_core::error::RpcError;
use starknet_types;
use thiserror::Error;
use tracing::error;

use super::WILDCARD_RPC_ERROR_CODE;

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
    #[error("Contract error: {msg}")]
    ContractError { msg: String },
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
    ValidationFailure,
}

impl ApiError {
    pub(crate) fn api_error_to_rpc_error(self) -> RpcError {
        match self {
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
                code: server::rpc_core::error::ErrorCode::ServerError(29),
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
            ApiError::ContractError { msg } => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(40),
                message: msg.into(),
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
            err @ ApiError::ClassAlreadyDeclared => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(51),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::InvalidContractClass => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(50),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::TypesError(_) => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::OnlyLatestBlock => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(24),
                message: err.to_string().into(),
                data: None,
            },
            ApiError::UnsupportedAction { msg } => RpcError {
                code: server::rpc_core::error::ErrorCode::InvalidRequest,
                message: msg.into(),
                data: None,
            },
            err @ ApiError::InsufficientMaxFee => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(53),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::InvalidTransactionNonce => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(52),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::InsufficientAccountBalance => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(54),
                message: err.to_string().into(),
                data: None,
            },
            err @ ApiError::ValidationFailure => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(55),
                message: err.to_string().into(),
                data: None,
            },
            ApiError::StarknetDevnetError(
                starknet_core::error::Error::TransactionValidationError(validation_error),
            ) => {
                let api_err = match validation_error {
                    starknet_core::error::TransactionValidationError::InsufficientMaxFee => ApiError::InsufficientMaxFee,
                    starknet_core::error::TransactionValidationError::InvalidTransactionNonce => ApiError::InvalidTransactionNonce,
                    starknet_core::error::TransactionValidationError::InsufficientAccountBalance => ApiError::InsufficientAccountBalance,
                    starknet_core::error::TransactionValidationError::GeneralFailure => ApiError::ValidationFailure,
                };

                api_err.api_error_to_rpc_error()
            }
            ApiError::StarknetDevnetError(error) => RpcError {
                code: server::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: error.to_string().into(),
                data: None,
            },
        }
    }
}

pub(crate) type RpcResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {

    use crate::api::json_rpc::error::{ApiError, RpcResult};
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
        error_expected_code_and_message(
            ApiError::ContractError { msg: "Contract error".into() },
            40,
            "Contract error",
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
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::GeneralFailure,
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            ApiError::ValidationFailure.api_error_to_rpc_error()
        );
        error_expected_code_and_message(
            ApiError::ValidationFailure,
            55,
            "Account validation failed",
        );
    }

    fn error_expected_code_and_message(err: ApiError, expected_code: i64, expected_message: &str) {
        let error_result = RpcResult::<()>::Err(err).to_rpc_result();
        match error_result {
            server::rpc_core::response::ResponseResult::Success(_) => panic!("Expected error"),
            server::rpc_core::response::ResponseResult::Error(err) => {
                assert_eq!(err.message, expected_message);
                assert_eq!(err.code, server::rpc_core::error::ErrorCode::ServerError(expected_code))
            }
        }
    }
}
