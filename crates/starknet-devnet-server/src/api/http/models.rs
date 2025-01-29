use serde::{Deserialize, Serialize};
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{Felt, Hash256, MsgToL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{
    BlockHash, Calldata, ClassHash, EntryPointSelector, Nonce, TransactionHash,
};
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::block::BlockId;
use starknet_types::rpc::eth_address::EthAddressWrapper;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transaction_receipt::FeeUnit;
use starknet_types::serde_helpers::dec_string::deserialize_biguint;

use crate::api::http::error::HttpApiError;
use crate::api::json_rpc::models::TransactionHashInput;
use crate::rpc_core::request::RpcMethodCall;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct DumpPath {
    pub path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct LoadPath {
    pub path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct PostmanLoadL1MessagingContract {
    pub network_url: String,
    pub address: Option<String>,
}

#[derive(Serialize)]
pub struct MessageHash {
    pub message_hash: Hash256,
}

// Implemented as type alias so JSON returned doesn't have extra key
pub type DumpResponseBody = Option<Vec<RpcMethodCall>>;

#[derive(Serialize)]
pub struct CreatedBlock {
    pub block_hash: BlockHash,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct AbortingBlocks {
    pub(crate) starting_block_id: BlockId,
}

#[derive(Serialize)]
pub struct AbortedBlocks {
    pub(crate) aborted: Vec<BlockHash>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct IncreaseTime {
    pub time: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct SetTime {
    pub time: u64,
    pub generate_block: Option<bool>,
}

#[derive(Serialize)]
pub struct SetTimeResponse {
    pub block_timestamp: u64,
    pub block_hash: Option<BlockHash>,
}

#[derive(Serialize)]
pub struct IncreaseTimeResponse {
    pub timestamp_increased_by: u64,
    pub block_hash: BlockHash,
}

#[derive(Serialize)]
pub struct SerializableAccount {
    pub initial_balance: String,
    pub address: ContractAddress,
    pub public_key: Felt,
    pub private_key: Felt,
    pub balance: Option<AccountBalancesResponse>,
}

#[derive(Serialize)]
pub struct AccountBalancesResponse {
    pub eth: AccountBalanceResponse,
    pub strk: AccountBalanceResponse,
}

#[derive(Serialize)]
pub struct AccountBalanceResponse {
    pub amount: String,
    pub unit: FeeUnit,
}

#[derive(Serialize)]
pub struct FeeToken {
    symbol: String,
    address: ContractAddress,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct MintTokensRequest {
    pub address: ContractAddress,
    #[serde(deserialize_with = "deserialize_biguint")]
    pub amount: BigUint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<FeeUnit>,
}

#[derive(Serialize)]
pub struct MintTokensResponse {
    /// decimal repr
    pub new_balance: String,
    pub unit: FeeUnit,
    pub tx_hash: TransactionHash,
}

#[derive(Serialize)]
pub struct ForkStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct FlushedMessages {
    pub messages_to_l1: Vec<MessageToL1>,
    pub messages_to_l2: Vec<MessageToL2>,
    pub generated_l2_transactions: Vec<TransactionHash>,
    pub l1_provider: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct FlushParameters {
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MessagingLoadAddress {
    pub messaging_contract_address: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct RestartParameters {
    pub restart_l1_to_l2_messaging: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct DebugTransactionRequest {
    pub contract_source: ContractSource,
    pub target: ExecutionTarget,
}

#[derive(Deserialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Debug))]
pub enum ContractSource {
    // workspace directory
    Path(LoadPath),
    // key is the filename with extension, value is the stringified content of the file
    Files(serde_json::Map<String, serde_json::Value>),
}

#[derive(Deserialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Debug))]
pub enum ExecutionTarget {
    TransactionHash(TransactionHashInput),
    // another case is to send: function selector, function arguments, sender address, receiver address
}

#[derive(Deserialize)]
#[serde(untagged)]
#[cfg_attr(test, derive(Debug))]
pub enum SierraArtifactSource {
    FilePath(LoadPath),
    SierraRepresentation(SierraClass),
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub struct WalnutVerificationRequest {
    pub contract_source: ContractSource,
    pub sierra_artifact_source: SierraArtifactSource,
}
