use blockifier::execution::stack_trace::{ErrorStackSegment, gen_tx_execution_error_trace};
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
        let error_string = value.to_string();
        let error_stack = gen_tx_execution_error_trace(&value);

        (error_stack, error_string).into()
    }
}

impl From<(blockifier::execution::stack_trace::ErrorStack, String)> for ContractExecutionError {
    fn from(
        (error_stack, error_string): (blockifier::execution::stack_trace::ErrorStack, String),
    ) -> Self {
        let mut recursive_error_option = Option::<ContractExecutionError>::None;
        fn format_error(stringified_error: &str, error_cause: &str) -> String {
            if stringified_error.is_empty() {
                error_cause.to_string()
            } else {
                format!("{} {}", stringified_error, error_cause)
            }
        }

        // [[[error inner] error outer] root]

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

                // VMException frame is ommitted, unless its the last frame of the error stack. It
                // doesnt produce any meaningful message to the developer
                ErrorStackSegment::Vm(vm) => recursive_error_option.take().unwrap_or(
                    ContractExecutionError::Message(format_error(&error_string, &String::from(vm))),
                ),
                ErrorStackSegment::StringFrame(msg) => {
                    ContractExecutionError::Message(format_error("", msg.as_str()))
                }
                ErrorStackSegment::EntryPoint(entry_point_error_frame) => {
                    let error_reason = match entry_point_error_frame.preamble_type {
                        blockifier::execution::stack_trace::PreambleType::CallContract => {
                            "Error in external contract"
                        }
                        blockifier::execution::stack_trace::PreambleType::LibraryCall => {
                            "Error in library call "
                        }
                        blockifier::execution::stack_trace::PreambleType::Constructor => {
                            "Error in constructor"
                        }
                    };

                    let error = recursive_error_option.take().unwrap_or_else(|| {
                        ContractExecutionError::Message(format_error(&error_string, error_reason))
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
            let error_msg = match error_stack.header {
                blockifier::execution::stack_trace::ErrorStackHeader::Constructor => {
                    "Constructor error"
                }
                blockifier::execution::stack_trace::ErrorStackHeader::Execution => {
                    "Execution error"
                }
                blockifier::execution::stack_trace::ErrorStackHeader::Validation => {
                    "Validation error"
                }
                blockifier::execution::stack_trace::ErrorStackHeader::None => "Unknown error",
            };

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

#[cfg(test)]
mod tests {

    use blockifier::execution::call_info::{CallInfo, Retdata};
    use blockifier::execution::stack_trace::{ErrorStack, ErrorStackSegment};
    use serde::{Deserialize, Serialize};
    use starknet_api::core::ContractAddress;
    use starknet_types_core::felt::Felt;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ContractErrorData {
        /// The execution trace up to the point of failure
        pub revert_error: ContractExecutionError,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct InnerContractExecutionError {
        pub contract_address: ContractAddress,
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

    impl From<ErrorStack> for ContractExecutionError {
        fn from(error_stack: ErrorStack) -> Self {
            let mut recursive_error_option = Option::<ContractExecutionError>::None;

            // [[[error inner] error outer] root]

            for frame in error_stack.stack.iter().rev() {
                let stack_err = match frame {
                    ErrorStackSegment::Cairo1RevertSummary(revert_summary) => {
                        let mut recursive_error = ContractExecutionError::Message(
                            serde_json::to_string(&revert_summary.last_retdata).unwrap_or_default(),
                        );

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
                    ErrorStackSegment::Vm(vm) => ContractExecutionError::Message(vm.into()),
                    ErrorStackSegment::StringFrame(msg) => {
                        ContractExecutionError::Message(msg.clone())
                    }
                    ErrorStackSegment::EntryPoint(entry_point_error_frame) => {
                        let error_reason = match entry_point_error_frame.preamble_type {
                            blockifier::execution::stack_trace::PreambleType::CallContract => {
                                "Error in external contract"
                            }
                            blockifier::execution::stack_trace::PreambleType::LibraryCall => {
                                "Error in library call "
                            }
                            blockifier::execution::stack_trace::PreambleType::Constructor => {
                                "Error in constructor"
                            }
                        };

                        let error = recursive_error_option.take().unwrap_or_else(|| {
                            ContractExecutionError::Message(error_reason.to_string())
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
                let error_msg = match error_stack.header {
                    blockifier::execution::stack_trace::ErrorStackHeader::Constructor => {
                        "Constructor error"
                    }
                    blockifier::execution::stack_trace::ErrorStackHeader::Execution => {
                        "Execution error"
                    }
                    blockifier::execution::stack_trace::ErrorStackHeader::Validation => {
                        "Validation error"
                    }
                    blockifier::execution::stack_trace::ErrorStackHeader::None => "Unknown error",
                };

                ContractExecutionError::Message(error_msg.to_string())
            }
        }
    }

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

    // impl From<CallInfo> for ContractExecutionError {
    //     fn from(value: CallInfo) -> Self {}
    // }

    #[test]
    fn des_snrs() {
        let js = r#"{
  "call": {
    "class_hash": "0x95fbeefe3200091edaeda57a08d76357e04d1a1ca878a4d1af777b9ce69af1",
    "code_address": null,
    "entry_point_type": "EXTERNAL",
    "entry_point_selector": "0x1017d0207c04f7ba93b6432c22f38693c884e40738af93551399097b24d0f27",
    "calldata": [
      "0x231560ec8712d5fd115eceb2be40b7186da134a32e2bc966fc63e5da48a72c2",
      "0x66756e6e795f74657874"
    ],
    "storage_address": "0x7500599919bcdd583f7ab259fdecddfe91df7b188aabd359652036bd27c671f",
    "caller_address": "0x0",
    "call_type": "Call",
    "initial_gas": 10000000000
  },
  "execution": {
    "retdata": [
      "0x526573756c743a3a756e77726170206661696c65642e"
    ],
    "events": [],
    "l2_to_l1_messages": [],
    "failed": true,
    "gas_consumed": 0
  },
  "inner_calls": [
    {
      "call": {
        "class_hash": "0x95fbeefe3200091edaeda57a08d76357e04d1a1ca878a4d1af777b9ce69af1",
        "code_address": "0x231560ec8712d5fd115eceb2be40b7186da134a32e2bc966fc63e5da48a72c2",
        "entry_point_type": "EXTERNAL",
        "entry_point_selector": "0x2f30ca48a88216d700251f5f06cb6dbdc3420bdef67879ba6836b55cf0d0dfd",
        "calldata": [
          "0x66756e6e795f74657874"
        ],
        "storage_address": "0x231560ec8712d5fd115eceb2be40b7186da134a32e2bc966fc63e5da48a72c2",
        "caller_address": "0x7500599919bcdd583f7ab259fdecddfe91df7b188aabd359652036bd27c671f",
        "call_type": "Call",
        "initial_gas": 10000000000
      },
      "execution": {
        "retdata": [
          "0x66756e6e795f74657874"
        ],
        "events": [],
        "l2_to_l1_messages": [],
        "failed": true,
        "gas_consumed": 0
      },
      "inner_calls": [],
      "resources": {
        "n_steps": 31,
        "n_memory_holes": 0,
        "builtin_instance_counter": {
          "range_check_builtin": 2
        }
      },
      "tracked_resource": "CairoSteps",
      "storage_read_values": [],
      "accessed_storage_keys": [],
      "read_class_hash_values": [],
      "accessed_contract_addresses": []
    }
  ],
  "resources": {
    "n_steps": 960,
    "n_memory_holes": 2,
    "builtin_instance_counter": {
      "range_check_builtin": 22
    }
  },
  "tracked_resource": "CairoSteps",
  "storage_read_values": [],
  "accessed_storage_keys": [],
  "read_class_hash_values": [],
  "accessed_contract_addresses": []
}"#;

        let call_info: CallInfo = serde_json::from_str(js).unwrap();

        let failed_calls = collect_failed_calls(&call_info);

        let mut recursive_error = Box::new(ContractExecutionError::Message(
            serde_json::to_string(
                &failed_calls
                    .iter()
                    .rev()
                    .flat_map(|f| f.execution.retdata.clone().0)
                    .collect::<Vec<Felt>>(),
            )
            .unwrap_or_default(),
        ));

        for f in failed_calls {
            let current = ContractExecutionError::Nested(InnerContractExecutionError {
                contract_address: f.call.storage_address,
                class_hash: f.call.class_hash.unwrap_or_default().0,
                selector: f.call.entry_point_selector.0,
                return_data: f.execution.retdata.clone(),
                error: recursive_error,
            });
            recursive_error = Box::new(current);
        }

        println!("{}", serde_json::to_string(&recursive_error).unwrap());
        println!("{}", serde_json::to_string(&ContractExecutionError::from(&call_info)).unwrap());
    }

    impl From<&CallInfo> for ContractExecutionError {
        fn from(call_info: &CallInfo) -> Self {
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

            for f in failed_calls {
                let current = ContractExecutionError::Nested(InnerContractExecutionError {
                    contract_address: f.call.storage_address,
                    class_hash: f.call.class_hash.unwrap_or_default().0,
                    selector: f.call.entry_point_selector.0,
                    return_data: f.execution.retdata.clone(),
                    error: Box::new(recursive_error),
                });
                recursive_error = current;
            }

            recursive_error
        }
    }
}
