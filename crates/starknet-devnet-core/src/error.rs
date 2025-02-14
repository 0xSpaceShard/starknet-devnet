use blockifier::fee::fee_checks::FeeCheckError;
use blockifier::transaction::errors::{
    TransactionExecutionError, TransactionFeeError, TransactionPreValidationError,
};
use starknet_rs_core::types::{BlockId, Felt};
use starknet_types;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_storage_key::ContractStorageKey;
use thiserror::Error;

use crate::stack_trace::{gen_tx_execution_error_trace, ErrorStack};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error(transparent)]
    StateError(#[from] StateError),
    #[error(transparent)]
    BlockifierStateError(#[from] blockifier::state::errors::StateError),
    #[error("{0:?}")]
    ContractExecutionError(ErrorStack),
    #[error("Execution error in simulating transaction no. {failure_index}: {error_stack:?}")]
    ContractExecutionErrorInSimulation { failure_index: usize, error_stack: ErrorStack },
    #[error("Types error: {0}")]
    TypesError(#[from] starknet_types::error::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Error when reading file {path}")]
    ReadFileError { source: std::io::Error, path: String },
    #[error("The file does not exist")]
    FileNotFound,
    #[error("Contract not found")]
    ContractNotFound,
    #[error(transparent)]
    SignError(#[from] starknet_rs_signers::local_wallet::SignError),
    #[error("{msg}")]
    InvalidMintingTransaction { msg: String },
    #[error("No block found")]
    NoBlock,
    #[error("No state at block {block_id:?}; consider running with --state-archive-capacity full")]
    NoStateAtBlock { block_id: BlockId },
    #[error("Format error")]
    FormatError,
    #[error("No transaction found")]
    NoTransaction,
    #[error("Invalid transaction index in a block")]
    InvalidTransactionIndexInBlock,
    #[error("Unsupported transaction type")]
    UnsupportedTransactionType,
    #[error("{msg}")]
    UnsupportedAction { msg: String },
    #[error("Unexpected internal error: {msg}")]
    UnexpectedInternalError { msg: String },
    #[error("Failed to load ContractClass: {0}")]
    ContractClassLoadError(String),
    #[error("Deserialization error: {origin}")]
    DeserializationError { origin: String },
    #[error("Serialization error: {origin}")]
    SerializationError { origin: String },
    #[error("Serialization not supported: {obj_name}")]
    SerializationNotSupported { obj_name: String },
    #[error(transparent)]
    TransactionValidationError(#[from] TransactionValidationError),
    #[error(transparent)]
    TransactionFeeError(blockifier::transaction::errors::TransactionFeeError),
    #[error(transparent)]
    MessagingError(#[from] MessagingError),
    #[error("Transaction has no trace")]
    NoTransactionTrace,
    #[error("the compiled class hash did not match the one supplied in the transaction")]
    CompiledClassHashMismatch,
    #[error("{msg}")]
    ClassAlreadyDeclared { msg: String },
    #[error("Requested entrypoint does not exist in the contract")]
    EntrypointNotFound,
}

impl From<starknet_types_core::felt::FromStrError> for Error {
    fn from(value: starknet_types_core::felt::FromStrError) -> Self {
        Self::UnexpectedInternalError { msg: value.to_string() }
    }
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("No class hash {0:x} found")]
    NoneClassHash(Felt),
    #[error("No compiled class hash found for class_hash {0:x}")]
    NoneCompiledHash(Felt),
    #[error("No casm class found for hash {0:x}")]
    NoneCasmClass(Felt),
    #[error("No contract state assigned for contact address: {0:x}")]
    NoneContractState(ContractAddress),
    #[error("No storage value assigned for: {0}")]
    NoneStorage(ContractStorageKey),
}

#[derive(Debug, Error)]
pub enum TransactionValidationError {
    #[error("The transaction's resources don't cover validation or the minimal transaction fee.")]
    InsufficientResourcesForValidate,
    #[error("Account transaction nonce is invalid.")]
    InvalidTransactionNonce,
    #[error("Account balance is not enough to cover the transaction cost.")]
    InsufficientAccountBalance,
    #[error("Account validation failed: {reason}")]
    ValidationFailure { reason: String },
}

impl From<TransactionExecutionError> for Error {
    fn from(value: TransactionExecutionError) -> Self {
        match value {
            TransactionExecutionError::TransactionPreValidationError(
                TransactionPreValidationError::InvalidNonce { .. },
            ) => TransactionValidationError::InvalidTransactionNonce.into(),
            TransactionExecutionError::FeeCheckError(err) => err.into(),
            TransactionExecutionError::TransactionPreValidationError(
                TransactionPreValidationError::TransactionFeeError(err),
            ) => err.into(),
            TransactionExecutionError::TransactionFeeError(err) => err.into(),
            TransactionExecutionError::ValidateTransactionError { .. } => {
                TransactionValidationError::ValidationFailure { reason: value.to_string() }.into()
            }
            err @ TransactionExecutionError::DeclareTransactionError { .. } => {
                Error::ClassAlreadyDeclared { msg: err.to_string() }
            }
            TransactionExecutionError::PanicInValidate { panic_reason } => {
                TransactionValidationError::ValidationFailure { reason: panic_reason.to_string() }
                    .into()
            }
            other => Self::ContractExecutionError(gen_tx_execution_error_trace(&other)),
        }
    }
}

impl From<FeeCheckError> for Error {
    fn from(value: FeeCheckError) -> Self {
        match value {
            FeeCheckError::MaxGasAmountExceeded { .. } | FeeCheckError::MaxFeeExceeded { .. } => {
                TransactionValidationError::InsufficientResourcesForValidate.into()
            }
            FeeCheckError::InsufficientFeeTokenBalance { .. } => {
                TransactionValidationError::InsufficientAccountBalance.into()
            }
        }
    }
}

impl From<TransactionFeeError> for Error {
    fn from(value: TransactionFeeError) -> Self {
        match value {
            TransactionFeeError::FeeTransferError { .. }
            | TransactionFeeError::MaxFeeTooLow { .. }
            | TransactionFeeError::MaxGasPriceTooLow { .. }
            | TransactionFeeError::MaxGasAmountTooLow { .. } => {
                TransactionValidationError::InsufficientResourcesForValidate.into()
            }
            TransactionFeeError::MaxFeeExceedsBalance { .. }
            | TransactionFeeError::GasBoundsExceedBalance { .. } => {
                TransactionValidationError::InsufficientAccountBalance.into()
            }
            err => Error::TransactionFeeError(err),
        }
    }
}

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error(
        "Message is not configured, ensure you've used `postman/load_l1_messaging_contract` \
         endpoint first."
    )]
    NotConfigured,
    #[error("An error has occurred during messages conversion: {0}.")]
    ConversionError(String),
    #[error("Ethers error: {0}.")]
    EthersError(String),
    #[error("Message to L1 with hash {0} is not present (never received OR already consumed).")]
    MessageToL1NotPresent(String),
    #[error("L1 not compatible: {0}")]
    IncompatibleL1(String),
}

pub type DevnetResult<T, E = Error> = Result<T, E>;
