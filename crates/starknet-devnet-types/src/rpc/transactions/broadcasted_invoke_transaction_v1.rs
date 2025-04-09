use serde::Deserialize;
use starknet_api::transaction::fields::Fee;

use crate::contract_address::ContractAddress;
use crate::felt::{Calldata, Nonce, TransactionSignature, TransactionVersion};
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl BroadcastedInvokeTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        calldata: &Calldata,
        version: TransactionVersion,
    ) -> Self {
        Self {
            sender_address,
            calldata: calldata.clone(),
            common: BroadcastedTransactionCommon {
                max_fee,
                signature: signature.clone(),
                nonce,
                version,
            },
        }
    }
}
