use serde::{Deserialize, Serialize};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash};
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
    #[serde(rename = "networkUrl")]
    pub network_url: String,
    pub address: Option<String>,
    #[serde(rename = "privateKey")]
    pub private_key: Option<String>,
}

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
    pub paid_fee_on_l1: Felt,
    pub nonce: Nonce,
}

impl TryFrom<MessageToL2> for L1HandlerTransaction {
    type Error = HttpApiError;

    fn try_from(msg: MessageToL2) -> Result<Self, Self::Error> {
        // The first argument of a `#l1_handler` Cairo function must be the address
        // of the L1 contract which have sent the message.
        let mut calldata = msg.payload.clone();
        calldata.insert(0, msg.l1_contract_address.into());

        let paid_fee_on_l1: u128 =
            msg.paid_fee_on_l1.try_into().map_err(|_| HttpApiError::InvalidValueError {
                msg: "paid_fee_on_l1 is out of range, expecting u128 value".to_string(),
            })?;

        Ok(L1HandlerTransaction {
            contract_address: msg.l2_contract_address,
            entry_point_selector: msg.entry_point_selector,
            calldata,
            nonce: msg.nonce,
            paid_fee_on_l1,
            ..Default::default()
        })
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
