use serde::{Deserialize, Serialize};
use starknet_types::felt::{Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
use starknet_types::starknet_api::transaction::Fee;

use crate::api::models::block::BlockHashHex;
use starknet_types::contract_address::ContractAddress;

#[derive(Deserialize, Debug)]
pub(crate) struct Path {
    path: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PostmanLoadL1MessagingContract {
    #[serde(rename = "networkUrl")]
    network_url: String,
    address: ContractAddress,
}

#[derive(Deserialize)]
pub(crate) struct MessageToL2 {
    l2_contract_address: ContractAddress,
    entry_point_selector: EntryPointSelector,
    l1_contract_addresss: ContractAddress,
    payload: Calldata,
    paid_fee_on_l1: Fee,
    nonce: Nonce,
}

#[derive(Deserialize)]
pub(crate) struct MessageFromL2 {
    l2_contract_address: ContractAddress,
    l1_contract_addresss: ContractAddress,
    payload: Calldata,
}

#[derive(Serialize)]
pub(crate) struct MessageHash {
    message_hash: Felt,
}

#[derive(Serialize)]
pub(crate) struct CreatedBlock {
    block_hash: BlockHashHex,
}

#[derive(Deserialize)]
pub(crate) struct AbortingBlocks {
    #[serde(rename = "startingBlockHash")]
    starting_block_hash: BlockHashHex,
}

#[derive(Serialize)]
pub(crate) struct AbortedBlocks {
    aborted: Vec<BlockHashHex>,
}

#[derive(Deserialize)]
pub(crate) struct Time {
    time: u64,
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
