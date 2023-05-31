use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StarknetInRustStateError(#[from] starknet_in_rust::core::errors::state_errors::StateError),
    #[error(transparent)]
    StarknetApiError(#[from] starknet_api::StarknetApiError),
    #[error("Error when calling python module")]
    PyModuleError,
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    JsonError(#[from] JsonError),
    #[error(transparent)]
    StarknetInRustContractAddressError(
        #[from] starknet_in_rust::core::errors::contract_address_errors::ContractAddressError,
    ),
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Byte array invalid")]
    FromByteArrayError,
}

#[derive(Error, Debug)]
pub enum JsonError {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Error: {msg}")]
    Custom { msg: String },
}
