use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::transaction::Fee;

use crate::api::models::{
    block::BlockHashHex,
    transaction::{Calldata, EntryPointSelectorHex, Nonce, TransactionHashHex},
    ContractAddressHex, FeltHex,
};

#[derive(Deserialize, Debug)]
pub(crate) struct Path {
    path: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PostmanLoadL1MessagingContract {
    #[serde(rename = "networkUrl")]
    network_url: String,
    address: ContractAddressHex,
}

#[derive(Deserialize)]
pub(crate) struct MessageToL2 {
    l2_contract_address: ContractAddressHex,
    entry_point_selector: EntryPointSelectorHex,
    l1_contract_addresss: ContractAddressHex,
    payload: Calldata,
    paid_fee_on_l1: Fee,
    nonce: Nonce,
}

#[derive(Deserialize)]
pub(crate) struct MessageFromL2 {
    l2_contract_address: ContractAddressHex,
    l1_contract_addresss: ContractAddressHex,
    payload: Calldata,
}

#[derive(Serialize)]
pub(crate) struct MessageHash {
    message_hash: FeltHex,
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
    pub(crate) address: String,
    pub(crate) public_key: String,
    pub(crate) private_key: String,
}

#[derive(Deserialize)]
pub(crate) struct ContractAddress {
    contract_address: ContractAddressHex,
}

#[derive(Serialize)]
pub(crate) struct Balance {
    amount: u128,
    unit: String,
}

#[derive(Serialize)]
pub(crate) struct FeeToken {
    symbol: String,
    address: ContractAddressHex,
}

#[derive(Deserialize)]
pub(crate) struct MintTokens {
    address: ContractAddressHex,
    amount: u128,
    lite: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct MintTokensResponse {
    new_balance: u128,
    unit: String,
    tx_hash: TransactionHashHex,
}

#[derive(Serialize)]
pub(crate) struct ForkStatus {
    url: String,
    block: u128,
}
