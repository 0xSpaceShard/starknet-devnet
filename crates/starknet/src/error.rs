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
    #[error("Specifying block by hash is currently not enabled")]
    BlockIdHashUnimplementedError,
    #[error("Specifying block by number is currently not enabled")]
    BlockIdNumberUnimplementedError,
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("Error when reading file {path}")]
    ReadFileError { source: std::io::Error, path: String },
    #[error("Contract not found")]
    ContractNotFound,
    #[error("No block found")]
    NoBlock,
    #[error("No state at block {block_number}")]
    NoStateAtBlock { block_number: u64},
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
