use blockifier::execution::stack_trace::{
    ErrorStack, ErrorStackHeader, ErrorStackSegment, PreambleType, gen_tx_execution_error_trace,
};
use blockifier::fee::fee_checks::FeeCheckError;
use blockifier::transaction::errors::{
    TransactionExecutionError, TransactionFeeError, TransactionPreValidationError,
};
use starknet_rs_core::types::{BlockId, Felt};
use starknet_types;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_storage_key::ContractStorageKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error(transparent)]
    StateError(#[from] StateError),
    #[error(transparent)]
    BlockifierStateError(#[from] blockifier::state::errors::StateError),
    #[error("{0:?}")]
    ContractExecutionError(ContractExecutionError),
    #[error("Execution error in simulating transaction no. {failure_index}: {execution_error:?}")]
    ContractExecutionErrorInSimulation {
        failure_index: usize,
        execution_error: ContractExecutionError,
    },
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
    #[error("Contract class size it too large")]
    ContractClassSizeIsTooLarge,
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
            other => Self::ContractExecutionError(other.into()),
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
        #[warn(clippy::wildcard_enum_match_arm)]
        match value {
            TransactionFeeError::FeeTransferError { .. }
            | TransactionFeeError::MaxFeeTooLow { .. }
            | TransactionFeeError::MaxGasPriceTooLow { .. }
            | TransactionFeeError::MaxGasAmountTooLow { .. } => {
                TransactionValidationError::InsufficientResourcesForValidate.into()
            }
            TransactionFeeError::MaxFeeExceedsBalance { .. }
            | TransactionFeeError::ResourcesBoundsExceedBalance { .. }
            | TransactionFeeError::GasBoundsExceedBalance { .. } => {
                TransactionValidationError::InsufficientAccountBalance.into()
            }
            err @ (TransactionFeeError::CairoResourcesNotContainedInFeeCosts
            | TransactionFeeError::ExecuteFeeTransferError(_)
            | TransactionFeeError::InsufficientFee { .. }
            | TransactionFeeError::MissingL1GasBounds
            | TransactionFeeError::StateError(_)) => Error::TransactionFeeError(err),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerContractExecutionError {
    pub contract_address: starknet_api::core::ContractAddress,
    pub class_hash: Felt,
    pub selector: Felt,
    #[serde(skip)]
    pub return_data: Retdata,
    pub error: Box<ContractExecutionError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContractExecutionError {
    /// Nested contract call stack trace frame.
    Nested(InnerContractExecutionError),
    /// Terminal error message.
    Message(String),
}

impl From<String> for ContractExecutionError {
    fn from(value: String) -> Self {
        ContractExecutionError::Message(value)
    }
}

use blockifier::execution::call_info::{CallInfo, Retdata};
use serde::{Deserialize, Serialize};

impl From<TransactionExecutionError> for ContractExecutionError {
    fn from(value: TransactionExecutionError) -> Self {
        let error_stack = gen_tx_execution_error_trace(&value);
        error_stack.into()
    }
}

fn preamble_type_to_error_msg(preamble_type: &PreambleType) -> &'static str {
    match preamble_type {
        PreambleType::CallContract => "Error in external contract",
        PreambleType::LibraryCall => "Error in library call",
        PreambleType::Constructor => "Error in constructor",
    }
}

fn header_to_error_msg(header: &ErrorStackHeader) -> &'static str {
    match header {
        ErrorStackHeader::Constructor => "Constructor error",
        ErrorStackHeader::Execution => "Execution error",
        ErrorStackHeader::Validation => "Validation error",
        ErrorStackHeader::None => "Unknown error",
    }
}

/// [[[error inner] error outer] root]
impl From<ErrorStack> for ContractExecutionError {
    fn from(error_stack: ErrorStack) -> Self {
        let error_string = error_stack.to_string();
        fn format_error(stringified_error: &str, error_cause: &str) -> String {
            if stringified_error.is_empty() {
                error_cause.to_string()
            } else {
                format!("{} {}", stringified_error, error_cause)
            }
        }

        let mut recursive_error_option = Option::<ContractExecutionError>::None;
        for frame in error_stack.stack.iter().rev() {
            let stack_err = match frame {
                ErrorStackSegment::Cairo1RevertSummary(revert_summary) => {
                    let mut recursive_error = ContractExecutionError::Message(format_error(
                        &error_string,
                        &serde_json::to_string(&revert_summary.last_retdata.0).unwrap_or_default(),
                    ));

                    for trace in revert_summary.stack.iter().rev() {
                        recursive_error =
                            ContractExecutionError::Nested(InnerContractExecutionError {
                                contract_address: trace.contract_address,
                                class_hash: trace.class_hash.unwrap_or_default().0,
                                selector: trace.selector.0,
                                return_data: revert_summary.last_retdata.clone(),
                                error: Box::new(recursive_error),
                            });
                    }

                    recursive_error
                }

                // VMException frame is omitted, unless it's the last frame of the error stack. It
                // doesn't produce any meaningful message to the developer.
                ErrorStackSegment::Vm(vm) => recursive_error_option.take().unwrap_or(
                    ContractExecutionError::Message(format_error(&error_string, &String::from(vm))),
                ),
                ErrorStackSegment::StringFrame(msg) => {
                    ContractExecutionError::Message(format_error("", msg.as_str()))
                }
                ErrorStackSegment::EntryPoint(entry_point_error_frame) => {
                    let error = recursive_error_option.take().unwrap_or_else(|| {
                        ContractExecutionError::Message(format_error(
                            &error_string,
                            preamble_type_to_error_msg(&entry_point_error_frame.preamble_type),
                        ))
                    });

                    ContractExecutionError::Nested(InnerContractExecutionError {
                        contract_address: entry_point_error_frame.storage_address,
                        class_hash: entry_point_error_frame.class_hash.0,
                        selector: entry_point_error_frame.selector.unwrap_or_default().0,
                        return_data: Retdata(Vec::new()),
                        error: Box::new(error),
                    })
                }
            };

            recursive_error_option = Some(stack_err);
        }

        if let Some(recursive_error) = recursive_error_option {
            recursive_error
        } else {
            let error_msg = header_to_error_msg(&error_stack.header);
            ContractExecutionError::Message(format_error(&error_string, error_msg))
        }
    }
}

impl From<&CallInfo> for ContractExecutionError {
    fn from(call_info: &CallInfo) -> Self {
        /// Traces recursively and returns elements starting from the deepest element
        /// and then moves outward to the enclosing elements
        fn collect_failed_calls(root_call: &CallInfo) -> Vec<&CallInfo> {
            let mut calls = vec![];

            for inner_call in root_call.inner_calls.iter() {
                calls.extend(collect_failed_calls(inner_call));
            }

            if root_call.execution.failed {
                calls.push(root_call);
            }

            calls
        }

        let failed_calls = collect_failed_calls(call_info);

        // collects retdata of each CallInfo, starting from the outermost element of failed_calls
        // collection and combines them in 1-dimensional array
        // It serves as the reason for the failed call stack trace
        let mut recursive_error = ContractExecutionError::Message(
            serde_json::to_string(
                &failed_calls
                    .iter()
                    .rev()
                    .flat_map(|f| f.execution.retdata.clone().0)
                    .collect::<Vec<Felt>>(),
            )
            .unwrap_or_default(),
        );

        for failed in failed_calls {
            let current = ContractExecutionError::Nested(InnerContractExecutionError {
                contract_address: failed.call.storage_address,
                class_hash: failed.call.class_hash.unwrap_or_default().0,
                selector: failed.call.entry_point_selector.0,
                return_data: failed.execution.retdata.clone(),
                error: Box::new(recursive_error),
            });
            recursive_error = current;
        }

        recursive_error
    }
}
