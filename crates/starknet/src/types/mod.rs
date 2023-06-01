use crate::error::Error;

pub(crate) mod contract_address;
pub(crate) mod contract_class;
pub(crate) mod contract_storage_key;
pub(crate) mod felt;

pub type DevnetResult<T> = Result<T, Error>;
