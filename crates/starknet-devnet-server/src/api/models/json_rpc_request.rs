use enum_helper_macros::{AllVariantsSerdeRenames, VariantName};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet_types::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::estimate_message_fee::EstimateMessageFeeRequest;
use starknet_types::rpc::gas_modification::GasModificationRequest;
use tracing::error;

use crate::api::endpoints_impl::accounts::{BalanceQuery, PredeployedAccountsQuery};
use crate::api::error::StrictRpcResult;
use crate::api::models::{
    AbortingBlocks, AcceptOnL1Request, AccountAddressInput, BlockAndClassHashInput,
    BlockAndContractAddressInput, BlockAndIndexInput, BlockIdInput,
    BroadcastedDeclareTransactionInput, BroadcastedDeployAccountTransactionInput,
    BroadcastedInvokeTransactionInput, CallInput, ClassHashInput, DumpPath, EstimateFeeInput,
    EventsInput, EventsSubscriptionInput, FlushParameters, GetStorageInput, GetStorageProofInput,
    IncreaseTime, JsonRpcResponse, L1TransactionHashInput, LoadPath, MintTokensRequest,
    PostmanLoadL1MessagingContract, RestartParameters, SetTime, SimulateTransactionsInput,
    SubscriptionBlockIdInput, SubscriptionIdInput, TransactionHashInput,
    TransactionReceiptSubscriptionInput, TransactionSubscriptionInput,
};
use crate::api::serde_helpers::{empty_params, optional_params};
use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::ResponseResult;

/// Helper trait to easily convert results to rpc results
pub trait ToRpcResponseResult {
    fn to_rpc_result(self) -> ResponseResult;
}

/// Used when there is no defined code to use
pub const WILDCARD_RPC_ERROR_CODE: i64 = -1;

/// Converts a serializable value into a `ResponseResult`
pub fn to_rpc_result<T: Serialize>(val: T) -> ResponseResult {
    match serde_json::to_value(val) {
        Ok(success) => ResponseResult::Success(success),
        Err(err) => {
            error!("Failed serialize rpc response: {:?}", err);
            ResponseResult::error(RpcError::internal_error())
        }
    }
}

impl ToRpcResponseResult for StrictRpcResult {
    fn to_rpc_result(self) -> ResponseResult {
        match self {
            Ok(JsonRpcResponse::Empty) => to_rpc_result(json!({})),
            Ok(JsonRpcResponse::Devnet(data)) => to_rpc_result(data),
            Ok(JsonRpcResponse::Starknet(data)) => to_rpc_result(data),
            Err(err) => err.api_error_to_rpc_error().into(),
        }
    }
}

#[derive(Deserialize, AllVariantsSerdeRenames, VariantName)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "method", content = "params")]
pub enum StarknetSpecRequest {
    #[serde(rename = "starknet_specVersion", with = "empty_params")]
    SpecVersion,
    #[serde(rename = "starknet_getBlockWithTxHashes")]
    BlockWithTransactionHashes(BlockIdInput),
    #[serde(rename = "starknet_getBlockWithTxs")]
    BlockWithFullTransactions(BlockIdInput),
    #[serde(rename = "starknet_getBlockWithReceipts")]
    BlockWithReceipts(BlockIdInput),
    #[serde(rename = "starknet_getStateUpdate")]
    StateUpdate(BlockIdInput),
    #[serde(rename = "starknet_getStorageAt")]
    StorageAt(GetStorageInput),
    #[serde(rename = "starknet_getStorageProof")]
    StorageProof(GetStorageProofInput),
    #[serde(rename = "starknet_getTransactionByHash")]
    TransactionByHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionByBlockIdAndIndex")]
    TransactionByBlockAndIndex(BlockAndIndexInput),
    #[serde(rename = "starknet_getTransactionReceipt")]
    TransactionReceiptByTransactionHash(TransactionHashInput),
    #[serde(rename = "starknet_getTransactionStatus")]
    TransactionStatusByHash(TransactionHashInput),
    #[serde(rename = "starknet_getMessagesStatus")]
    MessagesStatusByL1Hash(L1TransactionHashInput),
    #[serde(rename = "starknet_getClass")]
    ClassByHash(BlockAndClassHashInput),
    #[serde(rename = "starknet_getCompiledCasm")]
    CompiledCasmByClassHash(ClassHashInput),
    #[serde(rename = "starknet_getClassHashAt")]
    ClassHashAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getClassAt")]
    ClassAtContractAddress(BlockAndContractAddressInput),
    #[serde(rename = "starknet_getBlockTransactionCount")]
    BlockTransactionCount(BlockIdInput),
    #[serde(rename = "starknet_call")]
    Call(CallInput),
    #[serde(rename = "starknet_estimateFee")]
    EstimateFee(EstimateFeeInput),
    #[serde(rename = "starknet_blockNumber", with = "empty_params")]
    BlockNumber,
    #[serde(rename = "starknet_blockHashAndNumber", with = "empty_params")]
    BlockHashAndNumber,
    #[serde(rename = "starknet_chainId", with = "empty_params")]
    ChainId,
    #[serde(rename = "starknet_syncing", with = "empty_params")]
    Syncing,
    #[serde(rename = "starknet_getEvents")]
    Events(EventsInput),
    #[serde(rename = "starknet_getNonce")]
    ContractNonce(BlockAndContractAddressInput),
    #[serde(rename = "starknet_addDeclareTransaction")]
    AddDeclareTransaction(BroadcastedDeclareTransactionInput),
    #[serde(rename = "starknet_addDeployAccountTransaction")]
    AddDeployAccountTransaction(BroadcastedDeployAccountTransactionInput),
    #[serde(rename = "starknet_addInvokeTransaction")]
    AddInvokeTransaction(BroadcastedInvokeTransactionInput),
    #[serde(rename = "starknet_estimateMessageFee")]
    EstimateMessageFee(EstimateMessageFeeRequest),
    #[serde(rename = "starknet_simulateTransactions")]
    SimulateTransactions(SimulateTransactionsInput),
    #[serde(rename = "starknet_traceTransaction")]
    TraceTransaction(TransactionHashInput),
    #[serde(rename = "starknet_traceBlockTransactions")]
    BlockTransactionTraces(BlockIdInput),
}

#[derive(Deserialize, AllVariantsSerdeRenames, VariantName)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "method", content = "params")]
pub enum DevnetSpecRequest {
    #[serde(rename = "devnet_impersonateAccount")]
    ImpersonateAccount(AccountAddressInput),
    #[serde(rename = "devnet_stopImpersonateAccount")]
    StopImpersonateAccount(AccountAddressInput),
    #[serde(rename = "devnet_autoImpersonate", with = "empty_params")]
    AutoImpersonate,
    #[serde(rename = "devnet_stopAutoImpersonate", with = "empty_params")]
    StopAutoImpersonate,
    #[serde(rename = "devnet_dump", with = "optional_params")]
    Dump(Option<DumpPath>),
    #[serde(rename = "devnet_load")]
    Load(LoadPath),
    #[serde(rename = "devnet_postmanLoad")]
    PostmanLoadL1MessagingContract(PostmanLoadL1MessagingContract),
    #[serde(rename = "devnet_postmanFlush", with = "optional_params")]
    PostmanFlush(Option<FlushParameters>),
    #[serde(rename = "devnet_postmanSendMessageToL2")]
    PostmanSendMessageToL2(MessageToL2),
    #[serde(rename = "devnet_postmanConsumeMessageFromL2")]
    PostmanConsumeMessageFromL2(MessageToL1),
    #[serde(rename = "devnet_createBlock", with = "empty_params")]
    CreateBlock,
    #[serde(rename = "devnet_abortBlocks")]
    AbortBlocks(AbortingBlocks),
    #[serde(rename = "devnet_acceptOnL1")]
    AcceptOnL1(AcceptOnL1Request),
    #[serde(rename = "devnet_setGasPrice")]
    SetGasPrice(GasModificationRequest),
    #[serde(rename = "devnet_restart", with = "optional_params")]
    Restart(Option<RestartParameters>),
    #[serde(rename = "devnet_setTime")]
    SetTime(SetTime),
    #[serde(rename = "devnet_increaseTime")]
    IncreaseTime(IncreaseTime),
    #[serde(rename = "devnet_getPredeployedAccounts", with = "optional_params")]
    PredeployedAccounts(Option<PredeployedAccountsQuery>),
    #[serde(rename = "devnet_getAccountBalance")]
    AccountBalance(BalanceQuery),
    #[serde(rename = "devnet_mint")]
    Mint(MintTokensRequest),
    #[serde(rename = "devnet_getConfig", with = "empty_params")]
    DevnetConfig,
}

#[cfg_attr(test, derive(Debug))]
pub enum JsonRpcRequest {
    StarknetSpecRequest(StarknetSpecRequest),
    DevnetSpecRequest(DevnetSpecRequest),
    // If adding a new variant, expand `fn deserialize` and `fn all_variants_serde_renames`
}

impl<'de> Deserialize<'de> for JsonRpcRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_req = serde_json::Value::deserialize(deserializer)?;

        let method = raw_req.get("method").and_then(|m| m.as_str()).unwrap_or("<missing>");

        match method {
            method if method.starts_with("starknet_") => Ok(Self::StarknetSpecRequest(
                serde_json::from_value(raw_req).map_err(serde::de::Error::custom)?,
            )),
            method if method.starts_with("devnet_") => Ok(Self::DevnetSpecRequest(
                serde_json::from_value(raw_req).map_err(serde::de::Error::custom)?,
            )),
            invalid => Err(serde::de::Error::custom(format!("Invalid method: {invalid}"))),
        }
    }
}

impl StarknetSpecRequest {
    pub fn requires_notifying(&self) -> bool {
        #![warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_) => true,
            Self::SpecVersion
            | Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::ClassByHash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::ChainId
            | Self::Syncing
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::BlockTransactionTraces(_)
            | Self::StorageProof(_) => false,
        }
    }

    pub fn is_forwardable_to_origin(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::ClassByHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::StorageProof(_)
            | Self::BlockTransactionTraces(_) => true,
            Self::SpecVersion
            | Self::ChainId
            | Self::Syncing
            | Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_) => false,
        }
    }

    pub fn is_dumpable(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::AddDeclareTransaction(_)
            | Self::AddDeployAccountTransaction(_)
            | Self::AddInvokeTransaction(_) => true,
            Self::SpecVersion
            | Self::BlockWithTransactionHashes(_)
            | Self::BlockWithFullTransactions(_)
            | Self::BlockWithReceipts(_)
            | Self::StateUpdate(_)
            | Self::StorageAt(_)
            | Self::TransactionByHash(_)
            | Self::TransactionByBlockAndIndex(_)
            | Self::TransactionReceiptByTransactionHash(_)
            | Self::TransactionStatusByHash(_)
            | Self::ClassByHash(_)
            | Self::ClassHashAtContractAddress(_)
            | Self::ClassAtContractAddress(_)
            | Self::BlockTransactionCount(_)
            | Self::Call(_)
            | Self::EstimateFee(_)
            | Self::BlockNumber
            | Self::BlockHashAndNumber
            | Self::ChainId
            | Self::Syncing
            | Self::Events(_)
            | Self::ContractNonce(_)
            | Self::EstimateMessageFee(_)
            | Self::SimulateTransactions(_)
            | Self::TraceTransaction(_)
            | Self::BlockTransactionTraces(_)
            | Self::MessagesStatusByL1Hash(_)
            | Self::CompiledCasmByClassHash(_)
            | Self::StorageProof(_) => false,
        }
    }
}

impl DevnetSpecRequest {
    pub fn requires_notifying(&self) -> bool {
        #![warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::PostmanFlush(_)
            | Self::PostmanSendMessageToL2(_)
            | Self::CreateBlock
            | Self::AbortBlocks(_)
            | Self::AcceptOnL1(_)
            | Self::SetTime(_)
            | Self::IncreaseTime(_)
            | Self::Mint(_) => true,
            Self::ImpersonateAccount(_)
            | Self::StopImpersonateAccount(_)
            | Self::AutoImpersonate
            | Self::StopAutoImpersonate
            | Self::Dump(_)
            | Self::Load(_)
            | Self::PostmanLoadL1MessagingContract(_)
            | Self::PostmanConsumeMessageFromL2(_)
            | Self::SetGasPrice(_)
            | Self::Restart(_)
            | Self::PredeployedAccounts(_)
            | Self::AccountBalance(_)
            | Self::DevnetConfig => false,
        }
    }

    /// postmanFlush not dumped because it creates new RPC calls which get dumped
    pub fn is_dumpable(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::ImpersonateAccount(_)
            | Self::StopImpersonateAccount(_)
            | Self::AutoImpersonate
            | Self::StopAutoImpersonate
            | Self::PostmanLoadL1MessagingContract(_)
            | Self::PostmanSendMessageToL2(_)
            | Self::PostmanConsumeMessageFromL2(_)
            | Self::CreateBlock
            | Self::AbortBlocks(_)
            | Self::AcceptOnL1(_)
            | Self::SetGasPrice(_)
            | Self::SetTime(_)
            | Self::IncreaseTime(_)
            | Self::Mint(_) => true,
            Self::Dump(_)
            | Self::Load(_)
            | Self::PostmanFlush(_)
            | Self::Restart(_)
            | Self::PredeployedAccounts(_)
            | Self::AccountBalance(_)
            | Self::DevnetConfig => false,
        }
    }
}

impl JsonRpcRequest {
    pub fn requires_notifying(&self) -> bool {
        #![warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::StarknetSpecRequest(req) => req.requires_notifying(),
            Self::DevnetSpecRequest(req) => req.requires_notifying(),
        }
    }

    /// Should the request be retried by being forwarded to the forking origin?
    pub fn is_forwardable_to_origin(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::StarknetSpecRequest(req) => req.is_forwardable_to_origin(),
            Self::DevnetSpecRequest(_) => false,
        }
    }

    pub fn is_dumpable(&self) -> bool {
        #[warn(clippy::wildcard_enum_match_arm)]
        match self {
            Self::StarknetSpecRequest(req) => req.is_dumpable(),
            Self::DevnetSpecRequest(req) => req.is_dumpable(),
        }
    }

    pub fn all_variants_serde_renames() -> Vec<String> {
        let mut all_variants = vec![];
        for variants in [
            StarknetSpecRequest::all_variants_serde_renames(),
            DevnetSpecRequest::all_variants_serde_renames(),
        ] {
            all_variants.extend(variants);
        }
        all_variants
    }
}

pub enum JsonRpcWsRequest {
    OneTimeRequest(Box<JsonRpcRequest>),
    SubscriptionRequest(JsonRpcSubscriptionRequest),
}

impl<'de> Deserialize<'de> for JsonRpcWsRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_req = serde_json::Value::deserialize(deserializer)?;

        let method = raw_req.get("method").and_then(|m| m.as_str()).unwrap_or("<missing>");

        if method.starts_with("starknet_subscribe") || method == "starknet_unsubscribe" {
            Ok(Self::SubscriptionRequest(
                serde_json::from_value(raw_req).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(Self::OneTimeRequest(
                serde_json::from_value(raw_req).map_err(serde::de::Error::custom)?,
            ))
        }
    }
}

#[derive(Deserialize, AllVariantsSerdeRenames, VariantName)]
#[cfg_attr(test, derive(Debug))]
#[serde(tag = "method", content = "params")]
pub enum JsonRpcSubscriptionRequest {
    #[serde(rename = "starknet_subscribeNewHeads", with = "optional_params")]
    NewHeads(Option<SubscriptionBlockIdInput>),
    #[serde(rename = "starknet_subscribeTransactionStatus")]
    TransactionStatus(TransactionHashInput),
    #[serde(rename = "starknet_subscribeEvents")]
    Events(Option<EventsSubscriptionInput>),
    #[serde(rename = "starknet_subscribeNewTransactions", with = "optional_params")]
    NewTransactions(Option<TransactionSubscriptionInput>),
    #[serde(rename = "starknet_subscribeNewTransactionReceipts", with = "optional_params")]
    NewTransactionReceipts(Option<TransactionReceiptSubscriptionInput>),
    #[serde(rename = "starknet_unsubscribe")]
    Unsubscribe(SubscriptionIdInput),
}

pub fn to_json_rpc_request<D>(call: &RpcMethodCall) -> Result<D, RpcError>
where
    D: DeserializeOwned,
{
    let params: serde_json::Value = call.params.clone().into();
    let deserializable_call = json!({
        "method": call.method,
        "params": params
    });

    serde_json::from_value::<D>(deserializable_call).map_err(|err| {
        let err = err.to_string();
        // since JSON-RPC specification requires returning a Method Not Found error,
        // we apply a hacky way to decide - checking the stringified error message
        if err.contains("Invalid method") || err.contains(&format!("unknown variant `{}`", call.method)) {
            error!(target: "rpc", method = ?call.method, "failed to deserialize RPC call: unknown method");
            RpcError::method_not_found()
        } else {
            error!(target: "rpc", method = ?call.method, ?err, "failed to deserialize RPC call: invalid params");
            RpcError::invalid_params(err)
        }
    })
}

impl std::fmt::Display for JsonRpcRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant_name = match self {
            Self::StarknetSpecRequest(req) => req.variant_name(),
            Self::DevnetSpecRequest(req) => req.variant_name(),
        };
        write!(f, "{}", variant_name)
    }
}

#[cfg(test)]
mod requests_tests {

    use serde_json::json;
    use starknet_types::felt::felt_from_prefixed_hex;

    use super::{JsonRpcRequest, StarknetSpecRequest};
    use crate::rpc_core::request::RpcMethodCall;
    use crate::test_utils::{EXPECTED_INVALID_BLOCK_ID_MSG, assert_contains};

    #[test]
    fn deserialize_get_block_with_transaction_hashes_request() {
        let json_str =
            r#"{"method":"starknet_getBlockWithTxHashes","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pre_confirmed"));

        assert_deserialization_fails(&json_str.replace("latest", "0x134134"), "Invalid block ID");
    }

    #[test]
    fn deserialize_get_block_with_transactions_request() {
        let json_str = r#"{"method":"starknet_getBlockWithTxs","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pre_confirmed"));

        assert_deserialization_fails(
            json_str.replace("latest", "0x134134").as_str(),
            EXPECTED_INVALID_BLOCK_ID_MSG,
        );
    }

    #[test]
    fn deserialize_get_state_update_request() {
        let json_str = r#"{"method":"starknet_getStateUpdate","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(&json_str.replace("latest", "pre_confirmed"));

        assert_deserialization_fails(
            &json_str.replace("latest", "0x134134"),
            EXPECTED_INVALID_BLOCK_ID_MSG,
        );
    }

    #[test]
    fn deserialize_get_storage_at_request() {
        let json_str = r#"{"method":"starknet_getStorageAt","params":{"contract_address":"0x134134","key":"0x134134","block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            &json_str.replace(r#""contract_address":"0x134134""#, r#""contract_address":"123""#),
            "Missing prefix 0x in 123",
        );

        assert_deserialization_fails(
            &json_str.replace(r#""contract_address":"0x134134""#, r#""contract_address": 123"#),
            "invalid type: number, expected a string",
        );
    }

    #[test]
    fn deserialize_get_transaction_by_hash_request() {
        let json_str = r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x134134"}}"#;

        let request = serde_json::from_str::<JsonRpcRequest>(json_str).unwrap();

        match request {
            JsonRpcRequest::StarknetSpecRequest(StarknetSpecRequest::TransactionByHash(input)) => {
                assert!(input.transaction_hash == felt_from_prefixed_hex("0x134134").unwrap());
            }
            _ => panic!("Wrong request type"),
        }

        // Errored json, there is no object just string is passed
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":"0x134134"}"#,
            "invalid type: string \"0x134134\", expected struct",
        );
        // Errored json, hash is not prefixed with 0x
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"134134"}}"#,
            "expected hex string to be prefixed by '0x'",
        );
        // Errored json, hex longer than 64 chars; misleading error message coming from dependency
        assert_deserialization_fails(
            r#"{"method":"starknet_getTransactionByHash","params":{"transaction_hash":"0x004134134134134134134134134134134134134134134134134134134134134134"}}"#,
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_transaction_by_block_and_index_request() {
        let json_str = r#"{"method":"starknet_getTransactionByBlockIdAndIndex","params":{"block_id":"latest","index":0}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace('0', "\"0x1\"").as_str(),
            "invalid type: string \"0x1\", expected u64",
        );
    }

    #[test]
    fn deserialize_get_transaction_receipt_request() {
        let json_str = r#"{"method":"starknet_getTransactionReceipt","params":{"transaction_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_class_request() {
        let json_str = r#"{"method":"starknet_getClass","params":{"block_id":"latest","class_hash":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "expected hex string to be prefixed by '0x'",
        );
    }

    #[test]
    fn deserialize_get_class_hash_at_request() {
        let json_str = r#"{"method":"starknet_getClassHashAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("0x", "").as_str(),
            "Error converting from hex string",
        );
    }

    #[test]
    fn deserialize_get_class_at_request() {
        let json_str = r#"{"method":"starknet_getClassAt","params":{"block_id":"latest","contract_address":"0xAAABB"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(json_str.replace("0x", "").as_str(), "Missing prefix 0x");
    }

    #[test]
    fn deserialize_get_block_transaction_count_request() {
        let json_str =
            r#"{"method":"starknet_getBlockTransactionCount","params":{"block_id":"latest"}}"#;
        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("latest", "0x134134").as_str(),
            EXPECTED_INVALID_BLOCK_ID_MSG,
        );
    }

    #[test]
    fn deserialize_call_request() {
        let json_str = r#"{
            "method":"starknet_call",
            "params":{
                "block_id":"latest",
                "request":{
                    "contract_address":"0xAAABB",
                    "entry_point_selector":"0x134134",
                    "calldata":["0x134134"]
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("starknet_call", "starknet_Call").as_str(),
            "unknown variant `starknet_Call`",
        );

        assert_deserialization_fails(
            json_str
                .replace(r#""contract_address":"0xAAABB""#, r#""contract_address":"123""#)
                .as_str(),
            "Error converting from hex string",
        );
        assert_deserialization_fails(
            json_str
                .replace(
                    r#""entry_point_selector":"0x134134""#,
                    r#""entry_point_selector":"134134""#,
                )
                .as_str(),
            "expected hex string to be prefixed by '0x'",
        );
        assert_deserialization_fails(
            json_str.replace(r#""calldata":["0x134134"]"#, r#""calldata":["123"]"#).as_str(),
            "expected hex string to be prefixed by '0x'",
        );
        assert_deserialization_fails(
            json_str.replace(r#""calldata":["0x134134"]"#, r#""calldata":[123]"#).as_str(),
            "invalid type: number, expected a 32 byte array ([u8;32]) or a hexadecimal string",
        );
    }

    #[test]
    fn deserialize_deploy_account_fee_estimation_request() {
        let json_str = r#"{
            "method":"starknet_estimateFee",
            "params":{
                "block_id":"latest",
                "simulation_flags": [],
                "request":[
                    {
                        "type":"DEPLOY_ACCOUNT",
                        "resource_bounds": {
                            "l1_gas": {
                                "max_amount": "0x1",
                                "max_price_per_unit": "0x2"
                            },
                            "l1_data_gas": {
                                "max_amount": "0x1",
                                "max_price_per_unit": "0x2"
                            },
                            "l2_gas": {
                                "max_amount": "0x1",
                                "max_price_per_unit": "0x2"
                            }
                        },
                        "tip": "0xabc",
                        "paymaster_data": [],
                        "version": "0x100000000000000000000000000000003",
                        "signature": ["0xFF", "0xAA"],
                        "nonce": "0x0",
                        "contract_address_salt": "0x01",
                        "class_hash": "0x01",
                        "constructor_calldata": ["0x01"],
                        "nonce_data_availability_mode": "L1",
                        "fee_data_availability_mode": "L1"
                    }
                ]
            }
        }"#;

        assert_deserialization_succeeds(json_str);

        assert_deserialization_fails(
            json_str.replace("estimateFee", "estimate_fee").as_str(),
            "unknown variant `starknet_estimate_fee`",
        );
    }

    fn sample_declare_v3_body() -> serde_json::Value {
        json!({
            "type":"DECLARE",
            "version": "0x3",
            "signature": [
                "0x2216f8f4d9abc06e130d2a05b13db61850f0a1d21891c7297b98fd6cc51920d",
                "0x6aadfb198bbffa8425801a2342f5c6d804745912114d5976f53031cd789bb6d"
            ],
            "resource_bounds": {
                "l1_gas": {
                    "max_amount": "0x1",
                    "max_price_per_unit": "0x2"
                },
                "l1_data_gas": {
                    "max_amount": "0x1",
                    "max_price_per_unit": "0x2"
                },
                "l2_gas": {
                    "max_amount": "0x1",
                    "max_price_per_unit": "0x2"
                }
            },
            "tip": "0xabc",
            "paymaster_data": [],
            "account_deployment_data": [],
            "nonce": "0x0",
            "compiled_class_hash":"0x63b33a5f2f46b1445d04c06d7832c48c48ad087ce0803b71f2b8d96353716ca",
            "sender_address":"0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "contract_class": {
                "sierra_program": ["0xAA", "0xBB"],
                "entry_points_by_type": {
                    "EXTERNAL": [{"function_idx":0,"selector":"0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320"},{"function_idx":1,"selector":"0x39e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695"}],
                    "L1_HANDLER": [],
                    "CONSTRUCTOR": [{"function_idx":2,"selector":"0x28ffe4ff0f226a9107253e17a904099aa4f63a02a5621de0576e5aa71bc5194"}]
                },
                "abi": "[{\"type\": \"function\", \"name\": \"constructor\", \"inputs\": [{\"name\": \"initial_balance\", \"type\": \"core::felt252\"}], \"outputs\": [], \"state_mutability\": \"external\"}, {\"type\": \"function\", \"name\": \"increase_balance\", \"inputs\": [{\"name\": \"amount1\", \"type\": \"core::felt252\"}, {\"name\": \"amount2\", \"type\": \"core::felt252\"}], \"outputs\": [], \"state_mutability\": \"external\"}, {\"type\": \"function\", \"name\": \"get_balance\", \"inputs\": [], \"outputs\": [{\"type\": \"core::felt252\"}], \"state_mutability\": \"view\"}]",
                "contract_class_version": "0.1.0"
            },
            "nonce_data_availability_mode": "L1",
            "fee_data_availability_mode": "L1"
        })
    }

    fn create_declare_request(tx: serde_json::Value) -> serde_json::Value {
        json!({
            "method":"starknet_addDeclareTransaction",
            "params":{
                "declare_transaction": tx
            }
        })
    }

    fn create_estimate_request(requests: &[serde_json::Value]) -> serde_json::Value {
        json!({
            "method": "starknet_estimateFee",
            "params": {
                "block_id": "latest",
                "simulation_flags": [],
                "request": requests
            }
        })
    }

    #[test]
    fn deserialize_declare_v3_fee_estimation_request() {
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v3_body()]).to_string(),
        );
        assert_deserialization_succeeds(
            &create_estimate_request(&[sample_declare_v3_body()]).to_string().replace(
                r#""version":"0x3""#,
                r#""version":"0x100000000000000000000000000000003""#,
            ),
        );
    }

    #[test]
    fn deserialize_get_events_request() {
        let json_str = r#"{
            "method":"starknet_getEvents",
            "params":{
                "filter":{
                    "chunk_size": 1,
                    "address":"0xAAABB",
                    "keys":[["0xFF"], ["0xAA"]],
                    "from_block": "latest",
                    "to_block": "pre_confirmed",
                    "continuation_token": "0x11"
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_succeeds(
            json_str.replace(r#""to_block": "pre_confirmed","#, "").as_str(),
        );

        assert_deserialization_fails(
            json_str.replace(r#""chunk_size": 1,"#, "").as_str(),
            "missing field `chunk_size`",
        );
    }

    #[test]
    fn deserialize_get_nonce_request() {
        let json_str = r#"{
            "method":"starknet_getNonce",
            "params":{
                "block_id":"latest",
                "contract_address":"0xAAABB"
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_fails(
            json_str.replace(r#""block_id":"latest","#, "").as_str(),
            "missing field `block_id`",
        );
    }

    #[test]
    fn deserialize_add_deploy_account_transaction_request() {
        let json_str = r#"{
            "method":"starknet_addDeployAccountTransaction",
            "params":{
                "deploy_account_transaction":{
                    "type":"DEPLOY_ACCOUNT",
                    "resource_bounds": {
                        "l1_gas": {
                            "max_amount": "0x1",
                            "max_price_per_unit": "0x2"
                        },
                        "l1_data_gas": {
                            "max_amount": "0x1",
                            "max_price_per_unit": "0x2"
                        },
                        "l2_gas": {
                            "max_amount": "0x1",
                            "max_price_per_unit": "0x2"
                        }
                    },
                    "tip": "0xabc",
                    "paymaster_data": [],
                    "version": "0x3",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "contract_address_salt": "0x01",
                    "class_hash": "0x01",
                    "constructor_calldata": ["0x01"],
                    "nonce_data_availability_mode": "L1",
                    "fee_data_availability_mode": "L1"
                }
            }
        }"#;

        assert_deserialization_succeeds(json_str);
        assert_deserialization_fails(
            json_str.replace(r#""class_hash": "0x01","#, "").as_str(),
            "missing field `class_hash`",
        );
    }

    #[test]
    fn deserialize_add_declare_transaction_v3_request() {
        assert_deserialization_succeeds(
            &create_declare_request(sample_declare_v3_body()).to_string(),
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v3_body())
                .to_string()
                .replace(r#""version":"0x3""#, r#""version":"0x123""#),
            "Invalid version of declare transaction: \"0x123\"",
        );

        assert_deserialization_fails(
            &create_estimate_request(&[sample_declare_v3_body()])
                .to_string()
                .replace("resource_bounds", "resourceBounds"),
            "Invalid declare transaction v3: missing field `resource_bounds`",
        );

        assert_deserialization_fails(
            &create_declare_request(sample_declare_v3_body())
                .to_string()
                .replace(r#""nonce":"0x0""#, r#""nonce":123"#),
            "Invalid declare transaction v3: invalid type: integer `123`",
        );
    }

    #[test]
    fn deseralize_chain_id_request() {
        for body in [
            json!({
                "method": "starknet_chainId",
                "params": {}
            }),
            json!({
                "method": "starknet_chainId",
                "params": []
            }),
            json!({
                "method": "starknet_chainId",
            }),
        ] {
            assert_deserialization_succeeds(body.to_string().as_str())
        }
    }

    #[test]
    fn deserialize_spec_version_request() {
        for body in [
            json!({
                "method": "starknet_specVersion",
                "params": {}
            }),
            json!({
                "method": "starknet_specVersion",
                "params": []
            }),
            json!({
                "method": "starknet_specVersion",
            }),
        ] {
            assert_deserialization_succeeds(body.to_string().as_str())
        }
    }

    #[test]
    fn deserialize_devnet_methods_with_optional_body() {
        for mut body in [
            json!({
                "method": "devnet_dump",
                "params": {}
            }),
            json!({
                "method":"devnet_dump",
            }),
            json!({
                "method":"devnet_dump",
                "params": {"path": "path"}
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
                "params": {"with_balance": true}
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
            }),
            json!({
                "method":"devnet_getPredeployedAccounts",
                "params": {}
            }),
            json!({
                "method":"devnet_postmanFlush",
                "params": {"dry_run": true}
            }),
            json!({
                "method":"devnet_postmanFlush",
            }),
            json!({
                "method":"devnet_postmanFlush",
                "params": {}
            }),
        ] {
            let mut json_rpc_object = json!({
                "jsonrpc": "2.0",
                "id": 1,
            });

            json_rpc_object.as_object_mut().unwrap().append(body.as_object_mut().unwrap());

            let RpcMethodCall { method, params, .. } =
                serde_json::from_value(json_rpc_object).unwrap();
            let params: serde_json::Value = params.into();
            let deserializable_call = json!({
                "method": &method,
                "params": params
            });

            assert_deserialization_succeeds(deserializable_call.to_string().as_str())
        }
    }

    fn assert_deserialization_succeeds(json_str: &str) {
        serde_json::from_str::<JsonRpcRequest>(json_str).unwrap();
    }

    fn assert_deserialization_fails(json_str: &str, expected_msg: &str) {
        match serde_json::from_str::<JsonRpcRequest>(json_str) {
            Err(err) => assert_contains(&err.to_string(), expected_msg).unwrap(),
            other => panic!("Invalid result: {other:?}"),
        }
    }
}
