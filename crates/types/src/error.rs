use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
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
