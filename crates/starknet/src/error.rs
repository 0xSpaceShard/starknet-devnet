use thiserror::Error;
use {starknet_in_rust, starknet_types};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error(transparent)]
    StateError(#[from] starknet_in_rust::core::errors::state_errors::StateError),
    #[error(transparent)]
    TransactionError(#[from] starknet_in_rust::transaction::error::TransactionError),
    #[error("Types error")]
    TypesError(#[from] starknet_types::error::Error),
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("Error when reading file {path}")]
    ReadFileError { source: std::io::Error, path: String },
    #[error("Contract not found")]
    ContractNotFound,
    #[error(transparent)]
    SyscallHandlerError(
        #[from] starknet_in_rust::syscalls::syscall_handler_errors::SyscallHandlerError,
    ),
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
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
