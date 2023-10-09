use broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use declare_transaction_v0v1::DeclareTransactionV0V1;
use declare_transaction_v2::DeclareTransactionV2;
use deploy_account_transaction::DeployAccountTransaction;
use deploy_transaction::DeployTransaction;
use invoke_transaction_v1::InvokeTransactionV1;
use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::{EthAddress, Fee};
use starknet_in_rust::execution::{CallInfo, L2toL1MessageInfo};
use starknet_rs_core::types::{BlockId, ExecutionResult, TransactionFinalityStatus};

use super::estimate_message_fee::FeeEstimateWrapper;
use super::transaction_receipt::MessageToL1;
use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::error::{ConversionError, Error};
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

    pub fn get_max_fee(&self) -> Fee {
        match self {
            Transaction::Declare(tx) => tx.get_max_fee(),
            Transaction::DeployAccount(tx) => tx.get_max_fee(),
            Transaction::Deploy(tx) => tx.get_max_fee(),
            Transaction::Invoke(tx) => tx.get_max_fee(),
            Transaction::L1Handler(tx) => tx.get_max_fee(),
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

    pub fn create_common_receipt(
        &self,
        transaction_events: &[Event],
        block_hash: Option<&BlockHash>,
        block_number: Option<BlockNumber>,
        execution_result: &ExecutionResult,
        finality_status: Option<TransactionFinalityStatus>,
    ) -> CommonTransactionReceipt {
        let r#type = self.get_type();

        let output = TransactionOutput {
            actual_fee: self.get_max_fee(),
            messages_sent: Vec::new(),
            events: transaction_events.to_vec(),
        };

        let maybe_pending_properties = MaybePendingProperties {
            block_number,
            block_hash: block_hash.cloned(),
            finality_status,
        };

        CommonTransactionReceipt {
            r#type,
            transaction_hash: *self.get_transaction_hash(),
            output,
            execution_status: execution_result.clone(),
            maybe_pending_properties,
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
    pub fn get_max_fee(&self) -> Fee {
        match self {
            DeclareTransaction::Version0(tx) => tx.get_max_fee(),
            DeclareTransaction::Version1(tx) => tx.get_max_fee(),
            DeclareTransaction::Version2(tx) => tx.get_max_fee(),
        }
    }

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
    pub fn get_max_fee(&self) -> Fee {
        self.max_fee
    }

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
    pub fn get_max_fee(&self) -> Fee {
        match self {
            InvokeTransaction::Version0(tx) => tx.get_max_fee(),
            InvokeTransaction::Version1(tx) => tx.get_max_fee(),
        }
    }

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
    pub fn get_max_fee(&self) -> Fee {
        Fee(0)
    }

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

#[derive(Debug, Clone, Serialize)]
pub struct EventsChunk {
    pub events: Vec<crate::emitted_event::EmittedEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
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

#[derive(Debug, Clone, Serialize)]
pub enum CallType {
    #[serde(rename = "LIBRARY_CALL")]
    LibraryCall,
    #[serde(rename = "CALL")]
    Call,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionInvocation {
    #[serde(flatten)]
    function_call: FunctionCall,
    caller_address: Felt,
    class_hash: Felt,
    entry_point_type: EntryPointType,
    call_type: CallType,
    result: Vec<Felt>,
    calls: Vec<FunctionInvocation>,
    events: Vec<Event>,
    messages: Vec<MessageToL1>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TransactionTrace {
    Invoke(InvokeTransactionTrace),
    Declare(DeclareTransactionTrace),
    DeployAccount(DeployAccountTransactionTrace),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reversion {
    pub revert_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ExecutionInvocation {
    Succeeded(FunctionInvocation),
    Reverted(Reversion),
}

#[derive(Debug, Clone, Serialize)]
pub struct InvokeTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub execution_invocation: ExecutionInvocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployAccountTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub constructor_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulatedTransaction {
    pub transaction_trace: TransactionTrace,
    pub fee_estimation: FeeEstimateWrapper,
}

impl TryFrom<L2toL1MessageInfo> for MessageToL1 {
    type Error = Error;

    fn try_from(value: L2toL1MessageInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            from_address: value.from_address.try_into()?,
            to_address: EthAddress::try_from(StarkFelt::from(Felt::from(value.to_address.0)))?,
            payload: value.payload.into_iter().map(|p| p.into()).collect(),
        })
    }
}

impl From<starknet_in_rust::execution::CallType> for CallType {
    fn from(value: starknet_in_rust::execution::CallType) -> Self {
        match value {
            starknet_in_rust::execution::CallType::Call => Self::Call,
            starknet_in_rust::execution::CallType::Delegate => Self::LibraryCall,
        }
    }
}

impl TryFrom<starknet_in_rust::execution::Event> for Event {
    type Error = Error;

    fn try_from(value: starknet_in_rust::execution::Event) -> Result<Self, Self::Error> {
        Ok(Self {
            from_address: value.from_address.try_into()?,
            keys: value.keys.into_iter().map(|k| k.into()).collect(),
            data: value.data.into_iter().map(|d| d.into()).collect(),
        })
    }
}

impl TryFrom<CallInfo> for FunctionInvocation {
    type Error = Error;

    fn try_from(call_info: CallInfo) -> Result<Self, Self::Error> {
        // done here because Result handling (e.g with ? operator) can't simply
        // be used in closure passed to .map(...)
        let mut internal_calls: Vec<FunctionInvocation> = vec![];
        for internal_call in call_info.internal_calls.clone() {
            internal_calls.push(internal_call.try_into()?);
        }

        let mut messages: Vec<MessageToL1> = vec![];
        for message in call_info.get_sorted_l2_to_l1_messages()? {
            messages.push(message.try_into()?);
        }

        let mut events: Vec<Event> = vec![];
        for event in call_info.get_sorted_events()? {
            events.push(event.try_into()?);
        }

        Ok(FunctionInvocation {
            function_call: FunctionCall {
                contract_address: call_info.contract_address.try_into()?,
                entry_point_selector: call_info
                    .entry_point_selector
                    .ok_or(ConversionError::InvalidInternalStructure(
                        "entry_point_selector is unexpectedly undefined".into(),
                    ))?
                    .into(),
                calldata: call_info.calldata.iter().map(|c| c.into()).collect(),
            },
            caller_address: call_info.caller_address.0.into(),
            class_hash: call_info
                .class_hash
                .ok_or(ConversionError::InvalidInternalStructure(
                    "class_hash is unexpectedly undefined".into(),
                ))?
                .into(),
            entry_point_type: (match call_info.entry_point_type {
                Some(starknet_in_rust::EntryPointType::External) => Ok(EntryPointType::External),
                Some(starknet_in_rust::EntryPointType::L1Handler) => Ok(EntryPointType::L1Handler),
                Some(starknet_in_rust::EntryPointType::Constructor) => {
                    Ok(EntryPointType::Constructor)
                }
                None => Err(ConversionError::InvalidInternalStructure(
                    "entry_point_type is unexpectedly undefined".into(),
                )),
            })?,
            call_type: call_info
                .call_type
                .ok_or(ConversionError::InvalidInternalStructure(
                    "call_type is unexpectedly undefined".into(),
                ))?
                .into(),
            result: call_info.retdata.iter().map(|r| r.into()).collect(),
            calls: internal_calls,
            events,
            messages,
        })
    }
}
