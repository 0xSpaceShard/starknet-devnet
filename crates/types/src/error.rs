use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error(transparent)]
    HashError(#[from] starknet_in_rust::core::errors::hash_errors::HashError),
    #[error(transparent)]
    SyscallErrorHandlerError(
        #[from] starknet_in_rust::syscalls::syscall_handler_errors::SyscallHandlerError,
    ),
    #[error(transparent)]
    StarknetFfConversionError(#[from] starknet_rs_ff::FromByteSliceError),
    #[error("Error when calling python module")]
    PyModuleError,
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    #[error(transparent)]
    JsonError(#[from] JsonError),
    #[error(transparent)]
    ContractAddressError(
        #[from] starknet_in_rust::core::errors::contract_address_errors::ContractAddressError,
    ),
    #[error(transparent)]
    TransactionError(#[from] starknet_in_rust::transaction::error::TransactionError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Byte array invalid")]
    FromByteArrayError,
    #[error("Invalid format")]
    InvalidFormat,
}

#[derive(Error, Debug)]
pub enum JsonError {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Error: {msg}")]
    Custom { msg: String },
}

pub type DevnetResult<T, E = Error> = Result<T, E>;
