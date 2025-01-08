use serde::Serialize;
use starknet_api::transaction::Fee;

use super::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
use crate::contract_address::ContractAddress;
use crate::felt::{Calldata, Nonce, TransactionSignature, TransactionVersion};

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(
    feature = "testing",
    derive(serde::Deserialize, PartialEq, Eq),
    serde(deny_unknown_fields)
)]
pub struct InvokeTransactionV1 {
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl InvokeTransactionV1 {
    pub fn new(broadcasted_txn: &BroadcastedInvokeTransactionV1) -> InvokeTransactionV1 {
        InvokeTransactionV1 {
            max_fee: broadcasted_txn.common.max_fee,
            version: broadcasted_txn.common.version,
            signature: broadcasted_txn.common.signature.clone(),
            nonce: broadcasted_txn.common.nonce,
            sender_address: broadcasted_txn.sender_address,
            calldata: broadcasted_txn.calldata.clone(),
        }
    }
}
