use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use crate::contract_address::ContractAddress;
use crate::error::{DevnetResult, Error};
use crate::felt::{
    Calldata, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct InvokeTransactionV1 {
    pub transaction_hash: TransactionHash,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl HashProducer for InvokeTransactionV1 {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(self.transaction_hash)
    }
}
