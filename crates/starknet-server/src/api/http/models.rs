use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{Hash256, MsgToL1};
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

#[derive(Deserialize, Serialize)]
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

impl TryFrom<L1HandlerTransaction> for MessageToL2 {
    type Error = HttpApiError;

    fn try_from(value: L1HandlerTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            l2_contract_address: value.contract_address,
            entry_point_selector: value.entry_point_selector,
            l1_contract_address: ContractAddress::new(value.calldata[0]).map_err(|_| {
                HttpApiError::InvalidValueError {
                    msg: "l1_contract_address does not fit into ContractAddress".to_string(),
                }
            })?,
            payload: value.calldata[1..].to_vec(),
            paid_fee_on_l1: value.paid_fee_on_l1.into(),
            nonce: value.nonce,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct MessageToL1 {
    l2_contract_address: ContractAddress,
    l1_contract_address: ContractAddress,
    payload: Calldata,
}

impl From<MessageToL1> for MsgToL1 {
    fn from(value: MessageToL1) -> Self {
        Self {
            from_address: value.l2_contract_address.into(),
            to_address: value.l1_contract_address.into(),
            payload: value.payload.into_iter().map(|f| f.into()).collect(),
        }
    }
}

impl TryFrom<MsgToL1> for MessageToL1 {
    type Error = HttpApiError;

    fn try_from(value: MsgToL1) -> Result<Self, Self::Error> {
        Ok(Self {
            l2_contract_address: ContractAddress::new(Felt::from(value.from_address)).map_err(
                |_| HttpApiError::InvalidValueError {
                    msg: "l2_contract_address does not fit into ContractAddress".to_string(),
                },
            )?,
            l1_contract_address: ContractAddress::new(Felt::from(value.to_address)).map_err(
                |_| HttpApiError::InvalidValueError {
                    msg: "l1_contract_address does not fit into ContractAddress".to_string(),
                },
            )?,
            payload: value.payload.into_iter().map(|f| f.into()).collect(),
        })
    }
}

#[derive(Serialize)]
pub(crate) struct MessageHash {
    #[serde(rename = "messageHash")]
    pub message_hash: Hash256,
}

#[derive(Serialize)]
pub(crate) struct TxHash {
    #[serde(rename = "transactionHash")]
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
    #[serde(rename = "messagesToL1")]
    pub messages_to_l1: Vec<MessageToL1>,
    #[serde(rename = "messagesToL2")]
    pub messages_to_l2: Vec<MessageToL2>,
    #[serde(rename = "l1Provider")]
    pub l1_provider: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FlushParameters {
    #[serde(rename = "dryRun")]
    pub dry_run: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MessagingLoadAddress {
    #[serde(rename = "messageContractAddress")]
    pub messaging_contract_address: String,
}
