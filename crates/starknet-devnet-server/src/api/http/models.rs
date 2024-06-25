use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{Hash256, MsgToL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::eth_address::EthAddressWrapper;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transaction_receipt::FeeUnit;
use starknet_types::serde_helpers::dec_string::deserialize_biguint;

use crate::api::http::error::HttpApiError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct DumpPath {
    pub path: Option<String>,
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

#[derive(Serialize)]
pub struct TxHash {
    pub transaction_hash: TransactionHash,
}

#[derive(Serialize)]
pub struct CreatedBlock {
    pub block_hash: BlockHash,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct AbortingBlocks {
    pub(crate) starting_block_hash: BlockHash,
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
    pub balances: Option<Vec<AccountBalanceResponse>>,
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
    pub dry_run: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct MessagingLoadAddress {
    pub messaging_contract_address: String,
}
