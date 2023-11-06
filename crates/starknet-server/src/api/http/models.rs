use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
use starknet_types::rpc::transactions::L1HandlerTransaction;

use crate::api::http::error::HttpApiError;
use crate::api::serde_helpers::U128HexOrDec;

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
    #[serde(rename = "networkUrl")]
    pub network_url: String,
    pub address: Option<String>,

    // TODO: THIS IS FOR TESTING PURPOSE ONLY.
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct MessageToL2 {
    #[serde(rename = "l2ContractAddress")]
    pub l2_contract_address: ContractAddress,
    #[serde(rename = "entryPointSelector")]
    pub entry_point_selector: EntryPointSelector,
    #[serde(rename = "l1ContractAddress")]
    pub l1_contract_address: ContractAddress,
    pub payload: Calldata,
    #[serde(rename = "paidFeeOnL1")]
    #[serde_as(as = "U128HexOrDec")]
    pub paid_fee_on_l1: u128,
    pub nonce: Nonce,
}

impl From<MessageToL2> for L1HandlerTransaction {
    fn from(msg: MessageToL2) -> Self {
        // The first argument of a `#l1_handler` Cairo function must be the address
        // of the L1 contract which have sent the message.
        let mut calldata = msg.payload.clone();
        calldata.insert(0, msg.l1_contract_address.into());

        L1HandlerTransaction {
            contract_address: msg.l2_contract_address,
            entry_point_selector: msg.entry_point_selector,
            calldata,
            nonce: msg.nonce,
            paid_fee_on_l1: msg.paid_fee_on_l1,
            ..Default::default()
        }
    }
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
