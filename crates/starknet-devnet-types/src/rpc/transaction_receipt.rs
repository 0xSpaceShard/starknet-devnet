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

#[derive(Debug, Clone, Serialize)] // TODO PartialEq, Eq?
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
    pub l1_gas: u64,
    pub l1_data_gas: u64,
    pub l2_gas: u64,
}

impl From<&blockifier::transaction::objects::TransactionExecutionInfo> for ExecutionResources {
    fn from(value: &blockifier::transaction::objects::TransactionExecutionInfo) -> Self {
        ExecutionResources {
            l1_gas: value.receipt.gas.l1_gas.0,
            l1_data_gas: value.receipt.da_gas.l1_data_gas.0,
            l2_gas: value.receipt.gas.l2_gas.0,
        }
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
