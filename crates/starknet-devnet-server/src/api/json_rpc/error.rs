use serde_json::json;
use starknet_core::error::{ContractExecutionError, TransactionValidationError};
use starknet_rs_core::types::Felt;
use starknet_types;
use starknet_types::felt::Nonce;
use starknet_types::starknet_api::core::ContractAddress;
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
    ContractError(ContractExecutionError),
    #[error("Transaction execution error")]
    TransactionExecutionError { failure_index: usize, execution_error: ContractExecutionError },
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
    #[error("{msg}")]
    UnsupportedAction { msg: String },
    #[error("Invalid transaction nonce")]
    InvalidTransactionNonce {
        address: ContractAddress,
        account_nonce: Nonce,
        incoming_tx_nonce: Nonce,
    },
    #[error("The transaction's resources don't cover validation or the minimal transaction fee")]
    InsufficientResourcesForValidate,
    #[error(
        "Account balance is smaller than the transaction's maximal fee (calculated as the sum of \
         each resource's limit x max price)"
    )]
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
    #[error("Requested entrypoint does not exist in the contract")]
    EntrypointNotFound,
    #[error("Cannot go back more than 1024 blocks")]
    TooManyBlocksBack,
    #[error("Invalid subscription id")]
    InvalidSubscriptionId,
    #[error("Devnet doesn't support storage proofs")] // slightly modified spec message
    StorageProofNotSupported,
    #[error("Contract class size is too large")]
    ContractClassSizeIsTooLarge,
    #[error("Minting reverted")]
    MintingReverted { tx_hash: Felt, revert_reason: Option<String> },
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
            ApiError::ContractError(contract_execution_error) => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(40),
                message: error_message.into(),
                data: Some(json!({
                    "revert_error": contract_execution_error
                })),
            },
            ApiError::TransactionExecutionError { execution_error, failure_index } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(41),
                message: error_message.into(),
                data: Some(serde_json::json!({
                    "transaction_index": failure_index,
                    "execution_error": execution_error,
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
            ApiError::InvalidTransactionNonce { address, account_nonce, incoming_tx_nonce } => {
                RpcError {
                    code: crate::rpc_core::error::ErrorCode::ServerError(52),
                    message: error_message.into(),
                    data: Some(json!(format!(
                        "Invalid transaction nonce of contract at address {address}. Account \
                         nonce: {account_nonce}; got: {incoming_tx_nonce}."
                    ))),
                }
            }
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
                    TransactionValidationError::InsufficientResourcesForValidate => {
                        ApiError::InsufficientResourcesForValidate
                    }
                    TransactionValidationError::InvalidTransactionNonce {
                        address,
                        account_nonce,
                        incoming_tx_nonce,
                    } => ApiError::InvalidTransactionNonce {
                        address: address.into(),
                        account_nonce: *account_nonce,
                        incoming_tx_nonce: *incoming_tx_nonce,
                    },
                    TransactionValidationError::InsufficientAccountBalance => {
                        ApiError::InsufficientAccountBalance
                    }
                    TransactionValidationError::ValidationFailure { reason } => {
                        ApiError::ValidationFailure { reason }
                    }
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
            ApiError::EntrypointNotFound => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(21),
                message: error_message.into(),
                data: None,
            },
            ApiError::TooManyBlocksBack => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(68),
                message: error_message.into(),
                data: None,
            },
            ApiError::InvalidSubscriptionId => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(66),
                message: error_message.into(),
                data: None,
            },
            ApiError::StorageProofNotSupported => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(42),
                message: error_message.into(),
                data: None,
            },
            ApiError::ContractClassSizeIsTooLarge => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(57),
                message: error_message.into(),
                data: None,
            },
            ApiError::MintingReverted { tx_hash, revert_reason: reason } => RpcError {
                code: crate::rpc_core::error::ErrorCode::ServerError(WILDCARD_RPC_ERROR_CODE),
                message: error_message.into(),
                data: Some(serde_json::json!({ "tx_hash": tx_hash, "revert_reason": reason })),
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
            | Self::ContractNotFound // Doesn't require forwarding, handled at state reader level
            | Self::InvalidTransactionIndexInBlock
            | Self::ContractError { .. }
            | Self::NoBlocks
            | Self::RequestPageSizeTooBig
            | Self::InvalidContinuationToken
            | Self::TooManyKeysInFilter
            | Self::ClassAlreadyDeclared
            | Self::InvalidContractClass
            | Self::UnsupportedAction { .. }
            | Self::InvalidTransactionNonce { .. }
            | Self::InsufficientAccountBalance
            | Self::ValidationFailure { .. }
            | Self::HttpApiError(_)
            | Self::EntrypointNotFound
            | Self::TransactionExecutionError { .. }
            | Self::TooManyBlocksBack
            | Self::InvalidSubscriptionId
            | Self::InsufficientResourcesForValidate
            | Self::StorageProofNotSupported
            | Self::ContractClassSizeIsTooLarge
            | Self::MintingReverted { .. }
            | Self::CompiledClassHashMismatch => false,
        }
    }
}

pub type StrictRpcResult = Result<JsonRpcResponse, ApiError>;

#[cfg(test)]
mod tests {
    use serde_json::json;
    use starknet_core::error::ContractExecutionError;
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::starknet_api::core::Nonce;

    use super::StrictRpcResult;
    use crate::api::json_rpc::ToRpcResponseResult;
    use crate::api::json_rpc::error::ApiError;
    use crate::rpc_core::error::{ErrorCode, RpcError};

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
            ApiError::ContractError(ContractExecutionError::Message("some_reason".to_string()));

        error_expected_code_and_message(api_error, 40, "Contract error");

        // check contract error data property
        let error =
            ApiError::ContractError(ContractExecutionError::Message("some_reason".to_string()))
                .api_error_to_rpc_error();

        let error_data = error.data.unwrap();
        assert_eq!(error_data["revert_error"].as_str().unwrap(), "some_reason");
    }

    #[test]
    fn transaction_execution_error() {
        error_expected_code_and_message(
            ApiError::TransactionExecutionError {
                failure_index: 0,
                execution_error: ContractExecutionError::Message("anything".to_string()),
            },
            41,
            "Transaction execution error",
        );

        error_expected_code_and_data(
            ApiError::TransactionExecutionError {
                failure_index: 1,
                execution_error: ContractExecutionError::Message("anything".to_string()),
            },
            41,
            &serde_json::json!({ "transaction_index": 1, "execution_error": "anything" }),
        );
    }

    #[test]
    fn invalid_transaction_nonce_error() {
        let devnet_error =
            ApiError::StarknetDevnetError(starknet_core::error::Error::TransactionValidationError(
                starknet_core::error::TransactionValidationError::InvalidTransactionNonce {
                    address: ContractAddress::zero(),
                    account_nonce: Nonce(Felt::ONE),
                    incoming_tx_nonce: Nonce(Felt::TWO),
                },
            ));

        assert_eq!(
            devnet_error.api_error_to_rpc_error(),
            RpcError {
                code: ErrorCode::ServerError(52),
                message: "Invalid transaction nonce".into(),
                data: Some(json!(
                    "Invalid transaction nonce of contract at address \
                     0x0000000000000000000000000000000000000000000000000000000000000000. Account \
                     nonce: 1; got: 2."
                ))
            }
        );
    }

    #[test]
    fn insufficient_resources_error() {
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
            "Account balance is smaller than the transaction's maximal fee (calculated as the sum \
             of each resource's limit x max price)",
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

    #[test]
    fn minting_reverted_error() {
        let revert_reason = String::from("some kind of reason");
        let devnet_error = ApiError::MintingReverted {
            tx_hash: Felt::ONE,
            revert_reason: Some(revert_reason.clone()),
        };

        error_expected_code_and_data(
            devnet_error,
            -1,
            &serde_json::json!({
                "tx_hash": "0x1",
                "revert_reason": revert_reason,
            }),
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
