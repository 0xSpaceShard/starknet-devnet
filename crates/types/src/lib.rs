pub mod contract_storage_key;
pub mod error;
pub mod patricia_key;
pub mod rpc;
pub mod serde_helpers;
pub mod traits;
mod utils;

pub type DevnetResult<T> = Result<T, crate::error::Error>;

// Re export libraries
pub use rpc::contract_address;
pub use rpc::contract_class;
pub use rpc::emitted_event;
pub use rpc::felt;
pub use {cairo_felt, num_bigint, num_integer, starknet_api};
