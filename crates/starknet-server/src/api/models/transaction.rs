use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::block::BlockNumber;
use starknet_types::starknet_api::transaction::{EthAddress, Fee};

use super::block::BlockHashHex;
use super::contract_class::{DeprecatedContractClass, SierraContractClass};
use super::{BlockId, ContractAddressHex, FeltHex};

pub type TransactionHashHex = FeltHex;
pub type ClassHashHex = FeltHex;
pub type Nonce = FeltHex;
pub type TransactionVersionHex = FeltHex;
pub type TransactionSignature = Vec<FeltHex>;
pub type CompiledClassHashHex = FeltHex;
pub type EntryPointSelectorHex = FeltHex;
pub type Calldata = Vec<FeltHex>;
pub type ContractAddressSaltHex = FeltHex;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Transactions {
    Hashes(Vec<TransactionHashHex>),
    Full(Vec<TransactionWithType>),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionWithType {
    pub r#type: TransactionType,
    #[serde(flatten)]
    pub transaction: Transaction,
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
#[serde(untagged)]
pub enum Transaction {
    Declare(DeclareTransaction),
    DeployAccount(DeployAccountTransaction),
    Deploy(DeployTransaction),
    Invoke(InvokeTransaction),
    L1Handler(L1HandlerTransaction),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeclareTransactionV0V1 {
    pub class_hash: ClassHashHex,
    pub sender_address: ContractAddressHex,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub transaction_hash: TransactionHashHex,
    pub signature: TransactionSignature,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeclareTransactionV2 {
    pub class_hash: ClassHashHex,
    pub compiled_class_hash: CompiledClassHashHex,
    pub sender_address: ContractAddressHex,
    pub nonce: Nonce,
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub transaction_hash: TransactionHashHex,
    pub signature: TransactionSignature,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclareTransaction {
    Version0(DeclareTransactionV0V1),
    Version1(DeclareTransactionV0V1),
    Version2(DeclareTransactionV2),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct InvokeTransactionV0 {
    pub transaction_hash: TransactionHashHex,
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub contract_address: ContractAddressHex,
    pub entry_point_selector: EntryPointSelectorHex,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct InvokeTransactionV1 {
    pub transaction_hash: TransactionHashHex,
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub sender_address: ContractAddressHex,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InvokeTransaction {
    Version0(InvokeTransactionV0),
    Version1(InvokeTransactionV1),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployAccountTransaction {
    pub transaction_hash: TransactionHashHex,
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub class_hash: ClassHashHex,
    pub contract_address_salt: ContractAddressSaltHex,
    pub constructor_calldata: Calldata,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployTransaction {
    pub transaction_hash: TransactionHashHex,
    pub version: TransactionVersionHex,
    pub class_hash: ClassHashHex,
    pub contract_address_salt: ContractAddressSaltHex,
    pub constructor_calldata: Calldata,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct L1HandlerTransaction {
    pub transaction_hash: TransactionHashHex,
    pub version: TransactionVersionHex,
    pub nonce: Nonce,
    pub contract_address: ContractAddressHex,
    pub entry_point_selector: EntryPointSelectorHex,
    pub calldata: Calldata,
}

/// A transaction status in StarkNet.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
pub enum TransactionStatus {
    /// The transaction passed the validation and entered the pending block.
    #[serde(rename = "PENDING")]
    Pending,
    /// The transaction passed the validation and entered an actual created block.
    #[serde(rename = "ACCEPTED_ON_L2")]
    #[default]
    AcceptedOnL2,
    /// The transaction was accepted on-chain.
    #[serde(rename = "ACCEPTED_ON_L1")]
    AcceptedOnL1,
    /// The transaction failed validation.
    #[serde(rename = "REJECTED")]
    Rejected,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionReceiptWithStatus {
    pub status: TransactionStatus,
    #[serde(flatten)]
    pub receipt: TransactionReceipt,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TransactionReceipt {
    Deploy(DeployTransactionReceipt),
    Common(CommonTransactionReceipt),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DeployTransactionReceipt {
    #[serde(flatten)]
    pub common: CommonTransactionReceipt,
    pub contract_address: ContractAddressHex,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommonTransactionReceipt {
    pub transaction_hash: TransactionHashHex,
    pub r#type: TransactionType,
    pub block_hash: BlockHashHex,
    pub block_number: BlockNumber,
    #[serde(flatten)]
    pub output: TransactionOutput,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionOutput {
    pub actual_fee: Fee,
    pub messages_sent: Vec<MessageToL1>,
    pub events: Vec<Event>,
}

pub type L2ToL1Payload = Vec<FeltHex>;

/// An L2 to L1 message.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct MessageToL1 {
    pub from_address: ContractAddressHex,
    pub to_address: EthAddress,
    pub payload: L2ToL1Payload,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Event {
    pub from_address: ContractAddressHex,
    #[serde(flatten)]
    pub content: EventContent,
}

pub type EventKeyHex = FeltHex;
pub type EventData = Vec<FeltHex>;

/// An event content.
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventContent {
    pub keys: Vec<EventKeyHex>,
    pub data: EventData,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EventFilter {
    pub from_block: Option<BlockId>,
    pub to_block: Option<BlockId>,
    pub continuation_token: Option<String>,
    pub chunk_size: usize,
    pub address: Option<ContractAddressHex>,
    #[serde(default)]
    pub keys: Vec<HashSet<FeltHex>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EventsChunk {
    pub events: Vec<Event>,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct FunctionCall {
    pub contract_address: ContractAddressHex,
    pub entry_point_selector: EntryPointSelectorHex,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommon {
    pub max_fee: Fee,
    pub version: TransactionVersionHex,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedTransactionWithType {
    pub r#type: TransactionType,
    #[serde(flatten)]
    pub transaction: BroadcastedTransaction,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedTransaction {
    Invoke(BroadcastedInvokeTransaction),
    Declare(BroadcastedDeclareTransaction),
    DeployAccount(BroadcastedDeployAccountTransaction),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedInvokeTransaction {
    V0(BroadcastedInvokeTransactionV0),
    V1(BroadcastedInvokeTransactionV1),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedInvokeTransactionV0 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_address: ContractAddressHex,
    pub entry_point_selector: EntryPointSelectorHex,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedInvokeTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub sender_address: ContractAddressHex,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeclareTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_class: DeprecatedContractClass,
    pub sender_address: ContractAddressHex,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeclareTransactionV2 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddressHex,
    pub compiled_class_hash: CompiledClassHashHex,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeployAccountTransaction {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_address_salt: ContractAddressSaltHex,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHashHex,
}
