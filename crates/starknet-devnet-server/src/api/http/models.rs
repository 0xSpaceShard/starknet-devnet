use primitive_types::U256;
use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{Hash256, MsgToL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
use starknet_types::rpc::eth_address::EthAddressWrapper;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transaction_receipt::FeeUnits;
use starknet_types::rpc::transactions::L1HandlerTransaction;
use starknet_types::serde_helpers::dec_string::deserialize_u256;

use crate::api::http::error::HttpApiError;

#[derive(Deserialize, Debug)]
pub struct DumpPath {
    pub path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LoadPath {
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub struct PostmanLoadL1MessagingContract {
    pub network_url: String,
    pub address: Option<String>,
}

#[derive(Serialize)]
pub struct MessageHash {
    pub message_hash: Hash256,
}

#[derive(Serialize)]
pub struct TxHash {
    pub transaction_hash: TransactionHash,
}

#[derive(Serialize)]
pub struct CreatedBlock {
    pub block_hash: BlockHash,
}

#[derive(Deserialize)]
pub struct AbortingBlocks {
    #[serde(rename = "startingBlockHash")]
    starting_block_hash: BlockHash,
}

#[derive(Serialize)]
pub struct AbortedBlocks {
    aborted: Vec<BlockHash>,
}

#[derive(Deserialize)]
pub struct IncreaseTime {
    pub time: u64,
}

#[derive(Deserialize)]
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
}

#[derive(Serialize)]
pub struct Balance {
    amount: U256,
    unit: String,
}

#[derive(Serialize)]
pub struct FeeToken {
    symbol: String,
    address: ContractAddress,
}

#[derive(Debug, Deserialize)]
pub struct MintTokensRequest {
    pub address: ContractAddress,
    #[serde(deserialize_with = "deserialize_u256")]
    pub amount: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<FeeUnits>,
}

#[derive(Serialize)]
pub struct MintTokensResponse {
    /// decimal repr
    pub new_balance: String,
    pub unit: FeeUnits,
    pub tx_hash: TransactionHash,
}

#[derive(Serialize)]
pub struct ForkStatus {
    url: String,
    block: u128,
}

#[derive(Serialize)]
pub struct FlushedMessages {
    pub messages_to_l1: Vec<MessageToL1>,
    pub messages_to_l2: Vec<MessageToL2>,
    pub generated_l2_transactions: Vec<TransactionHash>,
    pub l1_provider: String,
}

#[derive(Serialize, Deserialize)]
pub struct FlushParameters {
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MessagingLoadAddress {
    pub messaging_contract_address: String,
}
