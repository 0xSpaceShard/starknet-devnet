use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use crate::error::{DevnetResult, Error};
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployAccountTransaction {
    pub transaction_hash: TransactionHash,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub class_hash: ClassHash,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
}

impl HashProducer for DeployAccountTransaction {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(self.transaction_hash)
    }
}
