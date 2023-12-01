use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{Hash256, MsgToL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
use starknet_types::rpc::eth_address::EthAddressWrapper;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transactions::L1HandlerTransaction;

use crate::api::http::error::HttpApiError;

#[derive(Deserialize, Debug)]
pub(crate) struct DumpPath {
    pub path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct LoadPath {
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PostmanLoadL1MessagingContract {
    pub network_url: String,
    pub address: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct MessageHash {
    pub message_hash: Hash256,
}

#[derive(Serialize)]
pub(crate) struct TxHash {
    pub transaction_hash: TransactionHash,
}

#[derive(Serialize)]
pub(crate) struct CreatedBlock {
    pub block_hash: BlockHash,
}

#[derive(Deserialize)]
pub(crate) struct AbortingBlocks {
    #[serde(rename = "startingBlockHash")]
    starting_block_hash: BlockHash,
}

#[derive(Serialize)]
pub(crate) struct AbortedBlocks {
    aborted: Vec<BlockHash>,
}

#[derive(Deserialize)]
pub(crate) struct Time {
    pub time: u64,
}

#[derive(Serialize)]
pub(crate) struct SetTimeResponse {
    pub block_timestamp: u64,
    pub block_hash: BlockHash,
}

#[derive(Serialize)]
pub(crate) struct IncreaseTimeResponse {
    pub timestamp_increased_by: u64,
    pub block_hash: BlockHash,
}

#[derive(Serialize)]
pub(crate) struct SerializableAccount {
    pub(crate) initial_balance: String,
    pub(crate) address: ContractAddress,
    pub(crate) public_key: Felt,
    pub(crate) private_key: Felt,
}

#[derive(Serialize)]
pub(crate) struct Balance {
    amount: u128,
    unit: String,
}

#[derive(Serialize)]
pub(crate) struct FeeToken {
    symbol: String,
    address: ContractAddress,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MintTokensRequest {
    pub(crate) address: ContractAddress,
    pub(crate) amount: u128,
}

#[derive(Serialize)]
pub(crate) struct MintTokensResponse {
    /// decimal repr
    pub(crate) new_balance: String,
    pub(crate) unit: String,
    pub(crate) tx_hash: TransactionHash,
}

#[derive(Serialize)]
pub(crate) struct ForkStatus {
    url: String,
    block: u128,
}

#[derive(Serialize)]
pub(crate) struct FlushedMessages {
    pub messages_to_l1: Vec<MessageToL1>,
    pub messages_to_l2: Vec<MessageToL2>,
    pub generated_l2_transactions: Vec<TransactionHash>,
    pub l1_provider: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FlushParameters {
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MessagingLoadAddress {
    pub messaging_contract_address: String,
}
