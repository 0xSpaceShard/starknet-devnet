use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, Felt, TransactionHash};
use crate::rpc::transactions::TransactionType;

use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::transaction::{EthAddress, Fee};
use starknet_rs_core::types::{
    ExecutionResult, MaybePendingTransactionReceipt, TransactionFinalityStatus,
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionReceiptWithStatus {
    #[serde(flatten)]
    pub receipt: TransactionReceipt,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionReceipt {
    Deploy(DeployTransactionReceipt),
    Common(CommonTransactionReceipt),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeployTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub contract_address: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTransactionReceipt {
    pub r#type: TransactionType,
    pub transaction_hash: TransactionHash,
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
    #[serde(flatten)]
    pub output: TransactionOutput,
    #[serde(flatten)]
    pub execution_status: ExecutionResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_status: Option<TransactionFinalityStatus>,
}

// impl From<TransactionReceiptWithStatus> for MaybePendingTransactionReceipt {
//     fn from(value: TransactionReceiptWithStatus) -> MaybePendingTransactionReceipt {
//         match value.receipt {
//             TransactionReceipt::Deploy()
//         }
//     }
// }

impl PartialEq for CommonTransactionReceipt {
    fn eq(&self, other: &Self) -> bool {
        let identical_execution_result = match (&self.execution_status, &other.execution_status) {
            (ExecutionResult::Succeeded, ExecutionResult::Succeeded) => true,
            (
                ExecutionResult::Reverted { reason: reason1 },
                ExecutionResult::Reverted { reason: reason2 },
            ) => reason1 == reason2,
            _ => false,
        };

        self.transaction_hash == other.transaction_hash
            && self.r#type == other.r#type
            && self.block_hash == other.block_hash
            && self.block_number == other.block_number
            && self.output == other.output
            && identical_execution_result
    }
}

impl Eq for CommonTransactionReceipt {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub actual_fee: Fee,
    pub messages_sent: Vec<MessageToL1>,
    pub events: Vec<Event>,
}

pub type L2ToL1Payload = Vec<Felt>;

/// An L2 to L1 message.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageToL1 {
    pub from_address: ContractAddress,
    pub to_address: EthAddress,
    pub payload: L2ToL1Payload,
}

#[cfg(test)]
mod tests {
    use crate::rpc::transaction_receipt::{
        CommonTransactionReceipt, TransactionReceipt, TransactionReceiptWithStatus,
    };
    use starknet_rs_core::types::{
        MaybePendingTransactionReceipt, TransactionReceipt as SRTransactionReceipt,
    };

    #[test]
    fn test_invoke_accepted_serialization() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/invoke_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let _: TransactionReceiptWithStatus = serde_json::from_str(&receipt).unwrap();
    }

    #[test]
    fn test_invoke_accepted_conversion() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/invoke_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let _: MaybePendingTransactionReceipt = serde_json::from_str(&receipt).unwrap();
    }

    #[test]
    fn test_declare_accepted_serialization() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/declare_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let _: TransactionReceiptWithStatus = serde_json::from_str(&receipt).unwrap();
    }
}
