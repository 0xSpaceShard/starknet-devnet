use serde::{Deserialize, Serialize};
use starknet_in_rust::definitions::constants::EXECUTE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::transaction::InvokeFunction as SirInvokeFunction;

use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{Calldata, Felt, TransactionHash};
use crate::rpc::transactions::invoke_transaction_v1::InvokeTransactionV1;
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedInvokeTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl BroadcastedInvokeTransactionV1 {
    // pub fn new(sender_address: ContractAddress, ) -> Self {
    //
    // }

    pub fn create_sir_invoke_function(&self, chain_id: &Felt) -> DevnetResult<SirInvokeFunction> {
        Ok(SirInvokeFunction::new(
            self.sender_address.into(),
            EXECUTE_ENTRY_POINT_SELECTOR.clone(),
            self.common.max_fee.0,
            self.common.version.into(),
            self.calldata.iter().map(|f| f.into()).collect(),
            self.common.signature.iter().map(|f| f.into()).collect(),
            chain_id.into(),
            Some(self.common.nonce.into()),
        )?)
    }

    pub fn create_invoke_transaction(
        &self,
        transaction_hash: &TransactionHash,
    ) -> InvokeTransactionV1 {
        InvokeTransactionV1 {
            transaction_hash: *transaction_hash,
            max_fee: self.common.max_fee,
            version: self.common.version,
            signature: self.common.signature.clone(),
            nonce: self.common.nonce,
            sender_address: self.sender_address,
            calldata: self.calldata.clone(),
        }
    }
}
