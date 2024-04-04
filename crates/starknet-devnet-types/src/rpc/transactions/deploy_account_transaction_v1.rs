use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_rs_core::types::BroadcastedDeployAccountTransaction;

use super::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use crate::contract_address::ContractAddress;
use crate::error::{DevnetResult, Error};
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::traits::HashProducer;

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployAccountTransactionV1 {
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub class_hash: ClassHash,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    #[serde(skip)]
    pub contract_address: ContractAddress,
}

impl DeployAccountTransactionV1 {
    pub fn new(
        broadcasted_txn: &BroadcastedDeployAccountTransactionV1,
        contract_address: ContractAddress,
    ) -> Self {
        Self {
            max_fee: broadcasted_txn.common.max_fee,
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
            nonce: broadcasted_txn.common.nonce,
            class_hash: broadcasted_txn.class_hash,
            contract_address_salt: broadcasted_txn.contract_address_salt,
            constructor_calldata: broadcasted_txn.constructor_calldata.clone(),
            contract_address,
        }
    }

    pub fn get_contract_address(&self) -> &ContractAddress {
        &self.contract_address
    }
}
