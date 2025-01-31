use blockifier::transaction::objects::ExecutionResourcesTraits;
use cairo_vm::types::builtin_name::BuiltinName;
use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::transaction::fields::Fee;
use starknet_rs_core::types::{ExecutionResult, Hash256, TransactionFinalityStatus};

use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, TransactionHash};
use crate::rpc::messaging::MessageToL1;
use crate::rpc::transactions::TransactionType;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
pub enum TransactionReceipt {
    Deploy(DeployTransactionReceipt),
    L1Handler(L1HandlerTransactionReceipt),
    Common(CommonTransactionReceipt),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
pub struct DeployTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub contract_address: ContractAddress,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
pub struct L1HandlerTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub message_hash: Hash256,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
pub struct MaybePendingProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<BlockNumber>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
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

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct ExecutionResources {
    #[serde(flatten)]
    pub computation_resources: ComputationResources,
    pub data_availability: DataAvailability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataAvailability {
    pub l1_gas: u64,
    pub l1_data_gas: u64,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
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
        let memory_holes = execution_info.receipt.resources.computation.vm_resources.n_memory_holes;

        let computation_resources = ComputationResources {
            steps: execution_info.receipt.resources.computation.vm_resources.total_n_steps(),
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
                l1_gas: execution_info.receipt.da_gas.l1_gas.0,
                l1_data_gas: execution_info.receipt.da_gas.l1_data_gas.0,
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
            .receipt
            .resources
            .computation
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

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct FeeAmount {
    pub amount: Fee,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize))]
#[serde(tag = "unit")]
pub enum FeeInUnits {
    WEI(FeeAmount),
    FRI(FeeAmount),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
