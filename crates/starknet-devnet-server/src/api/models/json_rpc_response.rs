#[cfg(test)]
use serde::Deserialize;
use serde::Serialize;
use starknet_core::CasmContractClass;
use starknet_rs_core::types::{ContractClass as CodegenContractClass, Felt};
use starknet_types::rpc::block::{Block, PreConfirmedBlock};
use starknet_types::rpc::estimate_message_fee::FeeEstimateWrapper;
use starknet_types::rpc::gas_modification::GasModification;
use starknet_types::rpc::state::{PreConfirmedStateUpdate, StateUpdate};
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::{
    BlockTransactionTrace, EventsChunk, L1HandlerTransactionStatus, SimulatedTransaction,
    TransactionStatus, TransactionTrace, TransactionWithHash,
};
use starknet_types::starknet_api::block::BlockNumber;

use crate::api::models::{
    AbortedBlocks, AcceptedOnL1Blocks, AccountBalanceResponse, BlockHashAndNumberOutput,
    CreatedBlock, DeclareTransactionOutput, DeployAccountTransactionOutput, DumpResponseBody,
    FlushedMessages, IncreaseTimeResponse, MessageHash, MessagingLoadAddress, MintTokensResponse,
    SerializableAccount, SetTimeResponse, SyncingOutput, TransactionHashOutput,
};
use crate::config::DevnetConfig;

#[derive(Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum JsonRpcResponse {
    Starknet(StarknetResponse),
    Devnet(DevnetResponse),
    Empty,
}

impl From<StarknetResponse> for JsonRpcResponse {
    fn from(resp: StarknetResponse) -> Self {
        JsonRpcResponse::Starknet(resp)
    }
}

impl From<DevnetResponse> for JsonRpcResponse {
    fn from(resp: DevnetResponse) -> Self {
        JsonRpcResponse::Devnet(resp)
    }
}

#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum StarknetResponse {
    Block(Block),
    PreConfirmedBlock(PreConfirmedBlock),
    StateUpdate(StateUpdate),
    PreConfirmedStateUpdate(PreConfirmedStateUpdate),
    Felt(Felt),
    Transaction(TransactionWithHash),
    TransactionReceiptByTransactionHash(Box<TransactionReceipt>),
    TransactionStatusByHash(TransactionStatus),
    ContractClass(CodegenContractClass),
    CompiledCasm(CasmContractClass),
    BlockTransactionCount(u64),
    Call(Vec<Felt>),
    EstimateFee(Vec<FeeEstimateWrapper>),
    BlockNumber(BlockNumber),
    BlockHashAndNumber(BlockHashAndNumberOutput),
    String(String),
    Syncing(SyncingOutput),
    Events(EventsChunk),
    AddDeclareTransaction(DeclareTransactionOutput),
    AddDeployAccountTransaction(DeployAccountTransactionOutput),
    TransactionHash(TransactionHashOutput),
    EstimateMessageFee(FeeEstimateWrapper),
    SimulateTransactions(Vec<SimulatedTransaction>),
    TraceTransaction(TransactionTrace),
    BlockTransactionTraces(Vec<BlockTransactionTrace>),
    MessagesStatusByL1Hash(Vec<L1HandlerTransactionStatus>),
}

#[derive(Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum DevnetResponse {
    MessagingContractAddress(MessagingLoadAddress),
    FlushedMessages(FlushedMessages),
    MessageHash(MessageHash),
    CreatedBlock(CreatedBlock),
    AbortedBlocks(AbortedBlocks),
    AcceptedOnL1Blocks(AcceptedOnL1Blocks),
    GasModification(GasModification),
    SetTime(SetTimeResponse),
    IncreaseTime(IncreaseTimeResponse),
    TransactionHash(TransactionHashOutput),
    PredeployedAccounts(Vec<SerializableAccount>),
    AccountBalance(AccountBalanceResponse),
    MintTokens(MintTokensResponse),
    DevnetConfig(DevnetConfig),
    DevnetDump(DumpResponseBody),
}

#[cfg(test)]
mod response_tests {
    use crate::api::error::StrictRpcResult;
    use crate::api::models::{JsonRpcResponse, ToRpcResponseResult};

    #[test]
    fn serializing_starknet_response_empty_variant_yields_empty_json_on_conversion_to_rpc_result() {
        assert_eq!(
            r#"{"result":{}}"#,
            serde_json::to_string(&StrictRpcResult::Ok(JsonRpcResponse::Empty).to_rpc_result())
                .unwrap()
        );
    }
}
