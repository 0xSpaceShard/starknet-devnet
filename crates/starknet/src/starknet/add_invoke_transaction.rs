use starknet_types::felt::{TransactionHash, Felt};
use crate::error::Result;
use super::Starknet;

pub fn add_invoke_transcation_v1(starknet: &mut Starknet) -> Result<TransactionHash>{
    Ok(Felt::from(0))
}