use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::transaction::DeployAccount as SirDeployAccount;

use crate::error::DevnetResult;
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeployAccountTransaction {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}

impl BroadcastedDeployAccountTransaction {
    pub fn new(
        constructor_calldata: &Calldata,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        class_hash: ClassHash,
        contract_address_salt: ContractAddressSalt,
        version: TransactionVersion,
    ) -> Self {
        Self {
            contract_address_salt,
            constructor_calldata: constructor_calldata.clone(),
            class_hash,
            common: BroadcastedTransactionCommon {
                max_fee,
                signature: signature.clone(),
                nonce,
                version,
            },
        }
    }
    // TODO: visibility & rename - create
    pub fn compile_sir_deploy_account(&self, chain_id: Felt) -> DevnetResult<SirDeployAccount> {
        Ok(SirDeployAccount::new(
            self.class_hash.bytes(),
            self.common.max_fee.0,
            self.common.version.into(),
            self.common.nonce.into(),
            self.constructor_calldata.iter().map(|h| h.into()).collect(),
            self.common.signature.iter().map(|h| h.into()).collect(),
            self.contract_address_salt.into(),
            chain_id.into(),
        )?)
    }

    pub fn compile_deploy_account_transaction(
        &self,
        transaction_hash: &TransactionHash,
    ) -> DeployAccountTransaction {
        DeployAccountTransaction {
            transaction_hash: *transaction_hash,
            max_fee: self.common.max_fee,
            version: self.common.version,
            signature: self.common.signature.clone(),
            nonce: self.common.nonce,
            class_hash: self.class_hash,
            contract_address_salt: self.contract_address_salt,
            constructor_calldata: self.constructor_calldata.clone(),
        }
    }
}
