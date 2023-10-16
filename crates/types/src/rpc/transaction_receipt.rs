use serde::{Deserialize, Serialize, Serializer};
use starknet_api::block::BlockNumber;
use starknet_api::core::EthAddress;
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{ExecutionResult, TransactionFinalityStatus};

use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, Felt, TransactionHash};
use crate::rpc::transactions::TransactionType;

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaybePendingProperties {
    #[serde(serialize_with = "serialize_finality_status")]
    pub finality_status: Option<TransactionFinalityStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<BlockNumber>,
}

pub fn serialize_finality_status<S>(
    finality_status: &Option<TransactionFinalityStatus>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let finality_status = finality_status.unwrap_or(TransactionFinalityStatus::AcceptedOnL2);
    finality_status.serialize(s)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTransactionReceipt {
    pub r#type: TransactionType,
    pub transaction_hash: TransactionHash,
    #[serde(flatten)]
    pub output: TransactionOutput,
    #[serde(flatten)]
    pub execution_status: ExecutionResult,
    #[serde(flatten)]
    pub maybe_pending_properties: MaybePendingProperties,
}

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
            && self.maybe_pending_properties == other.maybe_pending_properties
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
    use starknet_rs_core::types::MaybePendingTransactionReceipt;

    use crate::rpc::transaction_receipt::TransactionReceipt;

    #[test]
    fn test_invoke_accepted_serialization() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/invoke_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let _: TransactionReceipt = serde_json::from_str(&receipt).unwrap();
    }

    #[test]
    fn test_invoke_accepted_conversion() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/invoke_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let receipt: TransactionReceipt = serde_json::from_str(&receipt).unwrap();
        let serialized_receipt = serde_json::to_value(receipt).unwrap();
        let _: MaybePendingTransactionReceipt = serde_json::from_value(serialized_receipt).unwrap();
    }

    #[test]
    fn test_declare_accepted_serialization() {
        let receipt_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/declare_accepted.json");
        let receipt = std::fs::read_to_string(receipt_path).unwrap();

        let _: TransactionReceipt = serde_json::from_str(&receipt).unwrap();
    }
}
