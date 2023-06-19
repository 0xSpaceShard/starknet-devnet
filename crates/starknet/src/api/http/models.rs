use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct Path {
    path: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PostmanLoadL1MessagingContract {
    #[serde(rename = "networkUrl")]
    network_url: String,
    address: String,
}

#[derive(Serialize)]
pub(crate) struct Output {
    pub data: u32,
}

#[derive(Serialize)]
pub(crate) struct TransactionHash {
    pub(crate) transaction_hash: String,
}

#[derive(Deserialize)]
pub(crate) struct MessageToL2 {
    l2_contract_address: String,
    entry_point_selector: String,
    l1_contract_addresss: String,
    payload: Vec<String>,
    paid_fee_on_l1: String,
    nonce: String,
}

#[derive(Deserialize)]
pub(crate) struct MessageFromL2 {
    l2_contract_address: String,
    l1_contract_addresss: String,
    payload: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct MessageHash {
    message_hash: String,
}

#[derive(Serialize)]
pub(crate) struct CreatedBlock {
    block_hash: String,
}

#[derive(Deserialize)]
pub(crate) struct AbortingBlocks {
    #[serde(rename = "startingBlockHash")]
    starting_block_hash: String,
}

#[derive(Serialize)]
pub(crate) struct AbortedBlocks {
    aborted: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct Time {
    time: u64,
}

#[derive(Serialize)]
pub(crate) struct PredeployedAccount {
    initial_balance: u128,
    address: String,
    public_key: String,
    private_key: String,
}

#[derive(Deserialize)]
pub(crate) struct ContractAddress {
    contract_address: String,
}

#[derive(Serialize)]
pub(crate) struct ContractCode {
    program: String,
}

#[derive(Serialize)]
pub(crate) struct Balance {
    amount: u128,
    unit: String,
}

#[derive(Serialize)]
pub(crate) struct FeeToken {
    symbol: String,
    address: String,
}

#[derive(Deserialize)]
pub(crate) struct MintTokens {
    address: String,
    amount: u128,
    lite: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct MintTokensResponse {
    new_balance: u128,
    unit: String,
    tx_hash: String,
}

#[derive(Serialize)]
pub(crate) struct ForkStatus {
    url: String,
    block: u128,
}
