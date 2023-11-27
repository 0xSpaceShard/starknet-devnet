use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::core::EthAddress;
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{ExecutionResult, TransactionFinalityStatus};

use blockifier::execution::call_info::CallInfo;

use crate::contract_address::ContractAddress;
use crate::emitted_event::Event;
use crate::felt::{BlockHash, Felt, TransactionHash};
use crate::rpc::transactions::TransactionType;

// TODO: Move to trace.rs later
#[derive(Debug, Eq, PartialEq)] // // TODO: Add Serialize, Clone, Deserialize; Eq, PartialEq are needed?
// #[serde(untagged)] - unncomment? why it's there
pub enum TransactionTrace {
    Invoke(InvokeTransactionTrace),
    Declare(DeclareTransactionTrace),
    DeployAccount(DeployAccountTransactionTrace),
    L1Handler(L1HandlerTransactionTrace),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTransactionTrace {
    pub r#type: TransactionType,
    pub state_diff: bool, // TODO: Add state diff object types
}

#[derive(Debug, Eq, PartialEq)] // TODO: Add Serialize, Clone, Deserialize; Eq, PartialEq are needed?
pub struct InvokeTransactionTrace {
    pub validate_invocation: Option<CallInfo>,
    pub execute_invocation: Option<CallInfo>,
    pub fee_transfer_invocation: Option<CallInfo>,
    // #[serde(flatten)] // TODO: unncomment
    // pub common: CommonTransactionTrace,
}

#[derive(Debug, Eq, PartialEq)] // TODO: Add Serialize, Clone, Deserialize; Eq, PartialEq are needed?
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<CallInfo>,
    pub fee_transfer_invocation: Option<CallInfo>,
    // #[serde(flatten)]
    // pub common: CommonTransactionTrace,
}

#[derive(Debug, Eq, PartialEq)] // TODO: Add Serialize, Clone, Deserialize; Eq, PartialEq are needed?
pub struct DeployAccountTransactionTrace {
    // #[serde(flatten)]
    // pub trace: DeployAccountTransactionTrace,
    pub validate_invocation: Option<CallInfo>,
    pub constructor_invocation: Option<CallInfo>,
    pub fee_transfer_invocation: Option<CallInfo>,
    // #[serde(flatten)]
    // pub common: CommonTransactionTrace,
}

#[derive(Debug, Eq, PartialEq)] // TODO: Add Serialize, Clone, Deserialize; Eq, PartialEq are needed?
pub struct L1HandlerTransactionTrace {
    pub function_invocation: Option<CallInfo>,
    // #[serde(flatten)]
    // pub common: CommonTransactionTrace,
}
