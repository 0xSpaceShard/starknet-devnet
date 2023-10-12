use starknet_types;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::Felt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error(transparent)]
    StateError(#[from] StateError),
    #[error(transparent)]
    BlockifierStateError(#[from] blockifier::state::errors::StateError),
    #[error(transparent)]
    BlockifierTransactionError(#[from] blockifier::transaction::errors::TransactionExecutionError),
    #[error("Types error")]
    TypesError(#[from] starknet_types::error::Error),
    #[error("I/O error")]
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
    #[error("No state at block {block_number}")]
    NoStateAtBlock { block_number: u64 },
    #[error("Format error")]
    FormatError,
    #[error("Sierra compilation error")]
    SierraCompilationError,
    #[error("No transaction found")]
    NoTransaction,
    #[error("Invalid transaction index in a block")]
    InvalidTransactionIndexInBlock,
    #[error("{msg}")]
    UnsupportedAction { msg: String },
    #[error("Unexpected internal error: {msg}")]
    UnexpectedInternalError { msg: String },
    #[error("Failed to load ContractClass")]
    ContractClassLoadError,
    #[error("Deserialization error of {obj_name}")]
    DeserializationError { obj_name: String },
    #[error("Serialization error of {obj_name}")]
    SerializationError { obj_name: String },
    #[error("Serialization not supported")]
    SerializationNotSupported,
    #[error("{reason}")]
    FeeError { reason: String },
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

pub type DevnetResult<T, E = Error> = Result<T, E>;
