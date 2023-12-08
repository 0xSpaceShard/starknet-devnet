use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{ExecutionResult, TransactionFinalityStatus};

use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, Felt, TransactionHash};
use crate::rpc::messaging::MessageToL1;
use crate::rpc::transactions::TransactionType;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionReceipt {
    Deploy(DeployTransactionReceipt),
    Common(CommonTransactionReceipt),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub contract_address: ContractAddress,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaybePendingProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<BlockNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(deny_unknown_fields)]
pub struct CommonTransactionReceipt {
    pub r#type: TransactionType,
    pub transaction_hash: TransactionHash,
    #[serde(flatten)]
    pub output: TransactionOutput,
    #[serde(flatten)]
    pub execution_status: ExecutionResult,
    pub finality_status: TransactionFinalityStatus,
    #[serde(flatten)]
    pub maybe_pending_properties: MaybePendingProperties,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResources {
    pub steps: Felt,
    pub memory_holes: Felt,
    pub range_check_builtin_applications: Felt,
    pub pedersen_builtin_applications: Felt,
    pub poseidon_builtin_applications: Felt,
    pub ec_op_builtin_applications: Felt,
    pub ecdsa_builtin_applications: Felt,
    pub bitwise_builtin_applications: Felt,
    pub keccak_builtin_applications: Felt,
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
            && self.execution_resources == other.execution_resources
            && identical_execution_result
    }
}

impl Eq for CommonTransactionReceipt {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionOutput {
    pub actual_fee: Fee,
    pub messages_sent: Vec<MessageToL1>,
    pub events: Vec<Event>,
}
