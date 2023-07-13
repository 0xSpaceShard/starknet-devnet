use starknet_types;
use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    TypesError {
        #[from]
        source: starknet_types::error::Error,
        backtrace: Backtrace,
    },
}
