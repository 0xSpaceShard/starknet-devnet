use cairo_vm::types::builtin_name::BuiltinName;
use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{ExecutionResult, Hash256, TransactionFinalityStatus};

use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, TransactionHash};
use crate::rpc::messaging::MessageToL1;
use crate::rpc::transactions::TransactionType;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionReceipt {
    Deploy(DeployTransactionReceipt),
    L1Handler(L1HandlerTransactionReceipt),
    Common(CommonTransactionReceipt),
}

impl TransactionReceipt {
    pub fn get_block_number(&self) -> Option<u64> {
        match self {
            TransactionReceipt::Deploy(receipt) => &receipt.common,
            TransactionReceipt::L1Handler(receipt) => &receipt.common,
            TransactionReceipt::Common(receipt) => receipt,
        }
        .maybe_pending_properties
        .block_number
        .map(|BlockNumber(n)| n)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeployTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub contract_address: ContractAddress,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct L1HandlerTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub message_hash: Hash256,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MaybePendingProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<BlockNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonTransactionReceipt {
    pub r#type: TransactionType,
    pub transaction_hash: TransactionHash,
    pub actual_fee: FeeInUnits,
    pub messages_sent: Vec<MessageToL1>,
    pub events: Vec<Event>,
    #[serde(flatten)]
    pub execution_status: ExecutionResult,
    pub finality_status: TransactionFinalityStatus,
    #[serde(flatten)]
    pub maybe_pending_properties: MaybePendingProperties,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResources {
    #[serde(flatten)]
    pub computation_resources: ComputationResources,
    pub data_availability: DataAvailability,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DataAvailability {
    pub l1_gas: u128,
    pub l1_data_gas: u128,
}

/// custom implementation, because serde_json doesn't support deserializing to u128
/// if the struct is being used as a field in another struct that have #[serde(flatten)] or
/// #[serde(untagged)]
impl<'de> Deserialize<'de> for DataAvailability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json_obj =
            serde_json::Value::deserialize(deserializer).map_err(serde::de::Error::custom)?;

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct InnerDataAvailability {
            l1_gas: u128,
            l1_data_gas: u128,
        }

        let data_availability: InnerDataAvailability =
            serde_json::from_value(json_obj).map_err(serde::de::Error::custom)?;

        Ok(DataAvailability {
            l1_gas: data_availability.l1_gas,
            l1_data_gas: data_availability.l1_data_gas,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComputationResources {
    pub steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_holes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_check_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pedersen_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poseidon_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_op_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecdsa_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitwise_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keccak_builtin_applications: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_arena_builtin: Option<usize>,
}

impl From<&blockifier::execution::call_info::CallInfo> for ComputationResources {
    fn from(call_info: &blockifier::execution::call_info::CallInfo) -> Self {
        ComputationResources {
            steps: call_info.resources.n_steps,
            memory_holes: if call_info.resources.n_memory_holes == 0 {
                None
            } else {
                Some(call_info.resources.n_memory_holes)
            },
            range_check_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::range_check,
            ),
            pedersen_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::pedersen,
            ),
            poseidon_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::poseidon,
            ),
            ec_op_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::ec_op,
            ),
            ecdsa_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::ecdsa,
            ),
            bitwise_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::bitwise,
            ),
            keccak_builtin_applications: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::keccak,
            ),
            segment_arena_builtin: Self::get_resource_from_call_info(
                call_info,
                &BuiltinName::segment_arena,
            ),
        }
    }
}

impl From<&blockifier::transaction::objects::TransactionExecutionInfo> for ExecutionResources {
    fn from(execution_info: &blockifier::transaction::objects::TransactionExecutionInfo) -> Self {
        let memory_holes = execution_info.transaction_receipt.resources.vm_resources.n_memory_holes;

        let computation_resources = ComputationResources {
            steps: execution_info.transaction_receipt.resources.total_charged_steps(),
            memory_holes: if memory_holes == 0 { None } else { Some(memory_holes) },
            range_check_builtin_applications:
                ComputationResources::get_resource_from_execution_info(
                    execution_info,
                    &BuiltinName::range_check,
                ),
            pedersen_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::pedersen,
            ),
            poseidon_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::poseidon,
            ),
            ec_op_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::ec_op,
            ),
            ecdsa_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::ecdsa,
            ),
            bitwise_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::bitwise,
            ),
            keccak_builtin_applications: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::keccak,
            ),
            segment_arena_builtin: ComputationResources::get_resource_from_execution_info(
                execution_info,
                &BuiltinName::segment_arena,
            ),
        };

        Self {
            computation_resources,
            data_availability: DataAvailability {
                l1_gas: execution_info.transaction_receipt.da_gas.l1_gas,
                l1_data_gas: execution_info.transaction_receipt.da_gas.l1_data_gas,
            },
        }
    }
}

impl ComputationResources {
    fn get_resource_from_execution_info(
        execution_info: &blockifier::transaction::objects::TransactionExecutionInfo,
        resource_name: &BuiltinName,
    ) -> Option<usize> {
        execution_info
            .transaction_receipt
            .resources
            .vm_resources
            .builtin_instance_counter
            .get(resource_name)
            .cloned()
    }

    fn get_resource_from_call_info(
        call_info: &blockifier::execution::call_info::CallInfo,
        resource_name: &BuiltinName,
    ) -> Option<usize> {
        call_info.resources.builtin_instance_counter.get(resource_name).cloned()
    }
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
            && self.events == other.events
            && self.messages_sent == other.messages_sent
            && self.actual_fee == other.actual_fee
            && self.execution_resources == other.execution_resources
            && identical_execution_result
    }
}

impl Eq for CommonTransactionReceipt {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeeAmount {
    pub amount: Fee,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "unit")]
pub enum FeeInUnits {
    WEI(FeeAmount),
    FRI(FeeAmount),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum FeeUnit {
    WEI,
    FRI,
}

impl std::fmt::Display for FeeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FeeUnit::WEI => "WEI",
            FeeUnit::FRI => "FRI",
        })
    }
}
