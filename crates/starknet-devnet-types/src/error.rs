use blockifier::transaction::errors::TransactionExecutionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error("Conversion error: {0}")]
    ConversionError(#[from] ConversionError),
    #[error(transparent)]
    JsonError(#[from] JsonError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FromHexError(#[from] starknet_rs_core::types::eth_address::FromHexError),
    #[error(transparent)]
    TransactionExecutionError(#[from] TransactionExecutionError),
    // TODO import cairo-lang-starknet to the project so this error could be created with its
    // variants
    #[error("Sierra compilation error: {reason}")]
    SierraCompilationError { reason: String },
    #[error("Program error")]
    ProgramError,
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Byte array invalid")]
    FromByteArrayError,
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Invalid internal structure: {0}")]
    InvalidInternalStructure(String),
    #[error("Value is out of range: {0}")]
    OutOfRangeError(String),
    #[error("Error converting from hex string: {0}")]
    CustomFromHexError(String),
}

#[derive(Error, Debug)]
pub enum JsonError {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Error: {msg}")]
    Custom { msg: String },
}

pub type DevnetResult<T, E = Error> = Result<T, E>;
