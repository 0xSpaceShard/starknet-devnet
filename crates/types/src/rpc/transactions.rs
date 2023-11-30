use std::collections::HashMap;

use blockifier::execution::call_info::CallInfo;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::TransactionExecutionInfo;
use broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use declare_transaction_v0v1::DeclareTransactionV0V1;
use declare_transaction_v2::DeclareTransactionV2;
use deploy_account_transaction::DeployAccountTransaction;
use deploy_transaction::DeployTransaction;
use invoke_transaction_v1::InvokeTransactionV1;
use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{BlockId, ExecutionResult, TransactionFinalityStatus};

use super::estimate_message_fee::FeeEstimateWrapper;
use super::state::ThinStateDiff;
use super::transaction_receipt::{ExecutionResources, OrderedMessageToL1};
use crate::constants::{
    BITWISE_BUILTIN_NAME, EC_OP_BUILTIN_NAME, HASH_BUILTIN_NAME, KECCAK_BUILTIN_NAME, N_STEPS,
    POSEIDON_BUILTIN_NAME, RANGE_CHECK_BUILTIN_NAME, SIGNATURE_BUILTIN_NAME,
};
use crate::contract_address::ContractAddress;
use crate::emitted_event::{Event, OrderedEvent};
use crate::error::{ConversionError, DevnetResult};
use crate::felt::{
    BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transaction_receipt::{
    CommonTransactionReceipt, MaybePendingProperties, TransactionOutput,
};

pub mod broadcasted_declare_transaction_v1;
pub mod broadcasted_declare_transaction_v2;
pub mod broadcasted_deploy_account_transaction;
pub mod broadcasted_invoke_transaction;

pub mod declare_transaction_v0v1;
pub mod declare_transaction_v2;
pub mod deploy_account_transaction;
pub mod deploy_transaction;
pub mod invoke_transaction_v1;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Transactions {
    Hashes(Vec<TransactionHash>),
    Full(Vec<Transaction>),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
pub enum TransactionType {
    #[serde(rename(deserialize = "DECLARE", serialize = "DECLARE"))]
    Declare,
    #[serde(rename(deserialize = "DEPLOY", serialize = "DEPLOY"))]
    Deploy,
    #[serde(rename(deserialize = "DEPLOY_ACCOUNT", serialize = "DEPLOY_ACCOUNT"))]
    DeployAccount,
    #[serde(rename(deserialize = "INVOKE", serialize = "INVOKE"))]
    #[default]
    Invoke,
    #[serde(rename(deserialize = "L1_HANDLER", serialize = "L1_HANDLER"))]
    L1Handler,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Transaction {
    #[serde(rename = "DECLARE")]
    Declare(DeclareTransaction),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(DeployAccountTransaction),
    #[serde(rename = "DEPLOY")]
    Deploy(DeployTransaction),
    #[serde(rename = "INVOKE")]
    Invoke(InvokeTransaction),
    #[serde(rename = "L1_HANDLER")]
    L1Handler(L1HandlerTransaction),
}

impl Transaction {
    pub fn get_type(&self) -> TransactionType {
        match self {
            Transaction::Declare(_) => TransactionType::Declare,
            Transaction::DeployAccount(_) => TransactionType::DeployAccount,
            Transaction::Deploy(_) => TransactionType::Deploy,
            Transaction::Invoke(_) => TransactionType::Invoke,
            Transaction::L1Handler(_) => TransactionType::L1Handler,
        }
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            Transaction::Declare(tx) => tx.get_transaction_hash(),
            Transaction::L1Handler(tx) => tx.get_transaction_hash(),
            Transaction::DeployAccount(tx) => tx.get_transaction_hash(),
            Transaction::Invoke(tx) => tx.get_transaction_hash(),
            Transaction::Deploy(tx) => tx.get_transaction_hash(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_common_receipt(
        &self,
        transaction_events: &[Event],
        block_hash: Option<&BlockHash>,
        block_number: Option<BlockNumber>,
        execution_result: &ExecutionResult,
        finality_status: TransactionFinalityStatus,
        actual_fee: Fee,
        execution_info: &TransactionExecutionInfo,
    ) -> CommonTransactionReceipt {
        let r#type = self.get_type();

        fn get_memory_holes_from_call_info(call_info: &Option<CallInfo>) -> usize {
            if let Some(call) = call_info { call.vm_resources.n_memory_holes } else { 0 }
        }

        fn get_resource_from_execution_info(
            execution_info: &TransactionExecutionInfo,
            resource_name: &str,
        ) -> Felt {
            let resource =
                execution_info.actual_resources.0.get(resource_name).cloned().unwrap_or_default();
            Felt::from(resource as u128)
        }

        let total_memory_holes = get_memory_holes_from_call_info(&execution_info.execute_call_info)
            + get_memory_holes_from_call_info(&execution_info.validate_call_info)
            + get_memory_holes_from_call_info(&execution_info.fee_transfer_call_info);

        let execution_resources = ExecutionResources {
            steps: get_resource_from_execution_info(execution_info, N_STEPS),
            memory_holes: Felt::from(total_memory_holes as u128),
            range_check_builtin_applications: get_resource_from_execution_info(
                execution_info,
                RANGE_CHECK_BUILTIN_NAME,
            ),
            pedersen_builtin_applications: get_resource_from_execution_info(
                execution_info,
                HASH_BUILTIN_NAME,
            ),
            poseidon_builtin_applications: get_resource_from_execution_info(
                execution_info,
                POSEIDON_BUILTIN_NAME,
            ),
            ec_op_builtin_applications: get_resource_from_execution_info(
                execution_info,
                EC_OP_BUILTIN_NAME,
            ),
            ecdsa_builtin_applications: get_resource_from_execution_info(
                execution_info,
                SIGNATURE_BUILTIN_NAME,
            ),
            bitwise_builtin_applications: get_resource_from_execution_info(
                execution_info,
                BITWISE_BUILTIN_NAME,
            ),
            keccak_builtin_applications: get_resource_from_execution_info(
                execution_info,
                KECCAK_BUILTIN_NAME,
            ),
        };

        let output = TransactionOutput {
            actual_fee,
            messages_sent: Vec::new(), // TODO wrong
            events: transaction_events.to_vec(),
        };

        let maybe_pending_properties =
            MaybePendingProperties { block_number, block_hash: block_hash.cloned() };

        CommonTransactionReceipt {
            r#type,
            transaction_hash: *self.get_transaction_hash(),
            output,
            execution_status: execution_result.clone(),
            finality_status,
            maybe_pending_properties,
            execution_resources,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclareTransaction {
    Version0(DeclareTransactionV0V1),
    Version1(DeclareTransactionV0V1),
    Version2(DeclareTransactionV2),
}

impl DeclareTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            DeclareTransaction::Version0(tx) => tx.get_transaction_hash(),
            DeclareTransaction::Version1(tx) => tx.get_transaction_hash(),
            DeclareTransaction::Version2(tx) => tx.get_transaction_hash(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct InvokeTransactionV0 {
    pub transaction_hash: TransactionHash,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

impl InvokeTransactionV0 {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InvokeTransaction {
    Version0(InvokeTransactionV0),
    Version1(InvokeTransactionV1),
}

impl InvokeTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            InvokeTransaction::Version0(tx) => tx.get_transaction_hash(),
            InvokeTransaction::Version1(tx) => tx.get_transaction_hash(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct L1HandlerTransaction {
    pub transaction_hash: TransactionHash,
    pub version: TransactionVersion,
    pub nonce: Nonce,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

impl L1HandlerTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EventFilter {
    pub from_block: Option<BlockId>,
    pub to_block: Option<BlockId>,
    pub address: Option<ContractAddress>,
    pub keys: Option<Vec<Vec<Felt>>>,
    pub continuation_token: Option<String>,
    pub chunk_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsChunk {
    pub events: Vec<crate::emitted_event::EmittedEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionCall {
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommon {
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum BroadcastedTransaction {
    #[serde(rename = "INVOKE")]
    Invoke(BroadcastedInvokeTransaction),
    #[serde(rename = "DECLARE")]
    Declare(BroadcastedDeclareTransaction),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(BroadcastedDeployAccountTransaction),
}

impl BroadcastedTransaction {
    pub fn to_blockifier_account_transaction(
        &self,
        chain_id: Felt,
        only_query: bool,
    ) -> DevnetResult<blockifier::transaction::account_transaction::AccountTransaction> {
        let blockifier_transaction = match self {
            BroadcastedTransaction::Invoke(invoke_txn) => {
                let blockifier_invoke_txn =
                    invoke_txn.create_blockifier_invoke_transaction(chain_id, only_query)?;
                AccountTransaction::Invoke(blockifier_invoke_txn)
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(declare_v1)) => {
                let class_hash = declare_v1.generate_class_hash()?;
                let transaction_hash =
                    declare_v1.calculate_transaction_hash(&chain_id, &class_hash)?;
                AccountTransaction::Declare(
                    declare_v1.create_blockifier_declare(class_hash, transaction_hash)?,
                )
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(declare_v2)) => {
                AccountTransaction::Declare(declare_v2.create_blockifier_declare(chain_id)?)
            }
            BroadcastedTransaction::DeployAccount(deploy_account) => {
                AccountTransaction::DeployAccount(
                    deploy_account.create_blockifier_deploy_account(chain_id, only_query)?,
                )
            }
        };

        Ok(blockifier_transaction)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
}

impl<'de> Deserialize<'de> for BroadcastedDeclareTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let version_raw = value.get("version").ok_or(serde::de::Error::missing_field("version"))?;
        match version_raw.as_str() {
            Some("0x1") => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid declare transaction V1: {e}"))
                })?;
                Ok(BroadcastedDeclareTransaction::V1(Box::new(unpacked)))
            }
            Some("0x2") => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid declare transaction V2: {e}"))
                })?;
                Ok(BroadcastedDeclareTransaction::V2(Box::new(unpacked)))
            }
            _ => Err(serde::de::Error::custom(format!(
                "Invalid declare transaction version: {version_raw}"
            ))),
        }
    }
}

/// Flags that indicate how to simulate a given transaction.
/// By default, the sequencer behavior is replicated locally (enough funds are expected to be in the
/// account, and fee will be deducted from the balance before the simulation of the next
/// transaction). To skip the fee charge, use the SKIP_FEE_CHARGE flag.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub enum SimulationFlag {
    #[serde(rename = "SKIP_VALIDATE")]
    SkipValidate,
    #[serde(rename = "SKIP_FEE_CHARGE")]
    SkipFeeCharge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    #[serde(rename = "LIBRARY_CALL")]
    LibraryCall,
    #[serde(rename = "CALL")]
    Call,
    #[serde(rename = "DELEGATE")]
    Delegate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInvocation {
    #[serde(flatten)]
    function_call: FunctionCall,
    caller_address: ContractAddress,
    class_hash: Felt,
    entry_point_type: EntryPointType,
    call_type: CallType,
    result: Vec<Felt>,
    calls: Vec<FunctionInvocation>,
    events: Vec<OrderedEvent>,
    messages: Vec<OrderedMessageToL1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionTrace {
    #[serde(rename = "INVOKE")]
    Invoke(InvokeTransactionTrace),
    #[serde(rename = "DECLARE")]
    Declare(DeclareTransactionTrace),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(DeployAccountTransactionTrace),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reversion {
    pub revert_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecutionInvocation {
    Succeeded(FunctionInvocation),
    Reverted(Reversion),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub execute_invocation: ExecutionInvocation,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployAccountTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub constructor_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatedTransaction {
    pub transaction_trace: TransactionTrace,
    pub fee_estimation: FeeEstimateWrapper,
}

impl FunctionInvocation {
    pub fn try_from_call_info(
        mut call_info: blockifier::execution::call_info::CallInfo,
        address_to_class_hash: &HashMap<ContractAddress, Felt>,
    ) -> DevnetResult<Self> {
        let mut internal_calls: Vec<FunctionInvocation> = vec![];
        for internal_call in call_info.inner_calls {
            internal_calls.push(FunctionInvocation::try_from_call_info(
                internal_call,
                address_to_class_hash,
            )?);
        }

        // the logic for getting the sorted l2-l1 messages
        // is creating an array with enough room for all objects + 1
        // then based on the order we use this index

        call_info.execution.l2_to_l1_messages.sort_by_key(|msg| msg.order);

        let messages: Vec<OrderedMessageToL1> = call_info
            .execution
            .l2_to_l1_messages
            .into_iter()
            .map(|msg| OrderedMessageToL1::new(msg, call_info.call.caller_address.into()))
            .collect();

        call_info.execution.events.sort_by_key(|event| event.order);

        let events: Vec<OrderedEvent> = call_info
            .execution
            .events
            .into_iter()
            .map(|event| OrderedEvent::new(&event, call_info.call.storage_address.into()))
            .collect();

        let function_call = FunctionCall {
            contract_address: call_info.call.storage_address.into(),
            entry_point_selector: call_info.call.entry_point_selector.0.into(),
            calldata: call_info.call.calldata.0.iter().map(|f| Felt::from(*f)).collect(),
        };

        // call_info.call.class_hash could be None, so we deduce it from
        // call_info.call.storage_address which is function_call.contract_address
        let class_hash = if let Some(class_hash) = call_info.call.class_hash {
            class_hash.into()
        } else {
            address_to_class_hash
                .get(&function_call.contract_address)
                .ok_or(ConversionError::InvalidInternalStructure(
                    "class_hash is unexpectedly undefined".into(),
                ))
                .cloned()?
        };

        Ok(FunctionInvocation {
            function_call,
            caller_address: call_info.call.caller_address.into(),
            class_hash,
            entry_point_type: call_info.call.entry_point_type,
            call_type: match call_info.call.call_type {
                blockifier::execution::entry_point::CallType::Call => CallType::Call,
                blockifier::execution::entry_point::CallType::Delegate => CallType::Delegate,
            },
            result: call_info.execution.retdata.0.into_iter().map(Felt::from).collect(),
            calls: internal_calls,
            events,
            messages,
        })
    }
}
