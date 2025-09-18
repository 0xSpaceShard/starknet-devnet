mod json_rpc_request;
mod json_rpc_response;

pub use json_rpc_request::{
    DevnetSpecRequest, JsonRpcRequest, JsonRpcSubscriptionRequest, JsonRpcWsRequest,
    StarknetSpecRequest, ToRpcResponseResult, WILDCARD_RPC_ERROR_CODE, to_json_rpc_request,
};
pub use json_rpc_response::{DevnetResponse, JsonRpcResponse, StarknetResponse};
use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{Felt, Hash256, TransactionExecutionStatus};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, ClassHash, TransactionHash};
use starknet_types::num_bigint::BigUint;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{BlockId, SubscriptionBlockId};
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transaction_receipt::FeeUnit;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, BroadcastedTransaction, EventFilter, FunctionCall,
    SimulationFlag, TransactionFinalityStatus,
};
use starknet_types::serde_helpers::dec_string::deserialize_biguint;
use starknet_types::starknet_api::block::BlockNumber;

use crate::rpc_core::request::RpcMethodCall;
use crate::subscribe::{TransactionFinalityStatusWithoutL1, TransactionStatusWithoutL1};

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BlockIdInput {
    pub block_id: BlockId,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TransactionHashInput {
    pub transaction_hash: TransactionHash,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ClassHashInput {
    pub class_hash: ClassHash,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct GetStorageInput {
    pub contract_address: ContractAddress,
    pub key: PatriciaKey,
    pub block_id: BlockId,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContractStorage {
    pub contract_address: ContractAddress,
    pub storage_keys: Vec<Felt>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct GetStorageProofInput {
    pub block_id: BlockId,
    pub class_hashes: Option<Vec<Felt>>,
    pub contract_addresses: Option<Vec<ContractAddress>>,
    pub contracts_storage_keys: Option<Vec<ContractStorage>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BlockAndIndexInput {
    pub block_id: BlockId,
    pub index: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BlockAndClassHashInput {
    pub block_id: BlockId,
    pub class_hash: ClassHash,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct BlockAndContractAddressInput {
    pub block_id: BlockId,
    pub contract_address: ContractAddress,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AccountAddressInput {
    pub account_address: ContractAddress,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[serde(deny_unknown_fields)]
pub struct CallInput {
    pub request: FunctionCall,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateFeeInput {
    pub request: Vec<BroadcastedTransaction>,
    pub simulation_flags: Vec<SimulationFlag>,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(deny_unknown_fields)]
pub struct BlockHashAndNumberOutput {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(untagged)]
pub enum SyncingOutput {
    False(bool), // if it seems redundant, check the spec
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventsInput {
    pub filter: EventFilter,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BroadcastedDeclareTransactionEnumWrapper {
    #[serde(rename = "DECLARE")]
    Declare(BroadcastedDeclareTransaction),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionInput {
    pub declare_transaction: BroadcastedDeclareTransactionEnumWrapper,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionOutput {
    pub transaction_hash: TransactionHash,
    pub class_hash: ClassHash,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BroadcastedDeployAccountTransactionEnumWrapper {
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(BroadcastedDeployAccountTransaction),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionInput {
    pub deploy_account_transaction: BroadcastedDeployAccountTransactionEnumWrapper,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(deny_unknown_fields)]
pub struct DeployAccountTransactionOutput {
    pub transaction_hash: TransactionHash,
    pub contract_address: ContractAddress,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BroadcastedInvokeTransactionEnumWrapper {
    Invoke(BroadcastedInvokeTransaction),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionInput {
    pub invoke_transaction: BroadcastedInvokeTransactionEnumWrapper,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(deny_unknown_fields)]
pub struct TransactionHashOutput {
    pub transaction_hash: TransactionHash,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulateTransactionsInput {
    pub block_id: BlockId,
    pub transactions: Vec<BroadcastedTransaction>,
    pub simulation_flags: Vec<SimulationFlag>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
#[serde(deny_unknown_fields)]
pub struct TransactionStatusOutput {
    pub finality_status: TransactionFinalityStatus,
    pub execution_status: TransactionExecutionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L1TransactionHashInput {
    pub transaction_hash: Hash256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SubscriptionId(u64);

impl From<u64> for SubscriptionId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Serialize for SubscriptionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

/// Custom deserialization is needed, because subscriber initially received stringified u64 value.
impl<'de> Deserialize<'de> for SubscriptionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let u64_as_string = String::deserialize(deserializer)?;
        let subscription_id = u64_as_string.parse::<u64>().map_err(|_| {
            serde::de::Error::invalid_type(serde::de::Unexpected::Str(&u64_as_string), &"u64")
        })?;

        Ok(SubscriptionId(subscription_id))
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionIdInput {
    pub subscription_id: SubscriptionId,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionBlockIdInput {
    pub block_id: SubscriptionBlockId,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventsSubscriptionInput {
    pub block_id: Option<SubscriptionBlockId>,
    pub from_address: Option<ContractAddress>,
    pub keys: Option<Vec<Vec<Felt>>>,
    pub finality_status: Option<TransactionFinalityStatus>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TransactionSubscriptionInput {
    pub sender_address: Option<Vec<ContractAddress>>,
    pub finality_status: Option<Vec<TransactionStatusWithoutL1>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TransactionReceiptSubscriptionInput {
    pub sender_address: Option<Vec<ContractAddress>>,
    pub finality_status: Option<Vec<TransactionFinalityStatusWithoutL1>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct DumpPath {
    pub path: String,
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
    #[serde(alias = "address")]
    pub messaging_contract_address: Option<String>,
    pub deployer_account_private_key: Option<String>,
}

#[derive(Serialize)]
pub struct MessageHash {
    pub message_hash: Hash256,
}

// Implemented as type alias so JSON returned doesn't have extra key
pub type DumpResponseBody = Option<Vec<RpcMethodCall>>;

#[derive(Serialize)]
pub struct CreatedBlock {
    pub block_hash: BlockHash,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct AbortingBlocks {
    pub(crate) starting_block_id: BlockId,
}

#[derive(Serialize)]
pub struct AbortedBlocks {
    pub(crate) aborted: Vec<BlockHash>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct AcceptOnL1Request {
    pub(crate) starting_block_id: BlockId,
}

#[derive(Serialize)]
pub struct AcceptedOnL1Blocks {
    pub(crate) accepted: Vec<BlockHash>,
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
    pub balance: Option<AccountBalancesResponse>,
}

#[derive(Serialize)]
pub struct AccountBalancesResponse {
    pub eth: AccountBalanceResponse,
    pub strk: AccountBalanceResponse,
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
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MessagingLoadAddress {
    pub messaging_contract_address: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(Debug))]
pub struct RestartParameters {
    pub restart_l1_to_l2_messaging: bool,
}
#[cfg(test)]
mod tests {
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::patricia_key::PatriciaKey;
    use starknet_types::rpc::block::{BlockId, BlockTag};
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, BroadcastedTransaction,
    };

    use super::{BlockIdInput, EstimateFeeInput, GetStorageInput};
    use crate::test_utils::{EXPECTED_INVALID_BLOCK_ID_MSG, assert_contains};

    #[test]
    fn errored_deserialization_of_estimate_fee_with_broadcasted_declare_transaction() {
        // Errored json struct that passed DECLARE V3, but contract class is of type V1
        let json_str = r#"{
            "request": [{
                "type": "DECLARE",
                "version": "0x3",
                "signature": ["0xFF", "0xAA"],
                "nonce": "0x0",
                "sender_address": "0x0001",
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
                "compiled_class_hash": "0x01",
                "tip": "0xabc",
                "paymaster_data": [],
                "account_deployment_data": [],
                "contract_class": {
                    "abi": [{
                        "inputs": [],
                        "name": "getPublicKey",
                        "outputs": [
                            {
                                "name": "publicKey",
                                "type": "felt"
                            }
                        ],
                        "stateMutability": "view",
                        "type": "function"
                    },
                    {
                        "inputs": [],
                        "name": "setPublicKey",
                        "outputs": [
                            {
                                "name": "publicKey",
                                "type": "felt"
                            }
                        ],
                        "type": "function"
                    }],
                    "program": "",
                    "entry_points_by_type": {}
                },
                "nonce_data_availability_mode": "L1",
                "fee_data_availability_mode": "L1"
            }],
            "block_id": {
                "block_number": 1
            }
        }"#;

        match serde_json::from_str::<EstimateFeeInput>(json_str) {
            Err(err) => assert_contains(
                &err.to_string(),
                // error indicative of expecting a cairo1 class artifact
                "Invalid declare transaction v3: missing field `state_mutability`",
            )
            .unwrap(),
            other => panic!("Invalid result: {other:?}"),
        }
    }

    #[test]
    fn deserialize_estimate_fee_input() {
        let json_str = r#"{
            "request": [
                {
                    "type": "DECLARE",
                    "version": "0x3",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "sender_address": "0x0001",
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
                    "compiled_class_hash": "0x01",
                    "tip": "0xabc",
                    "paymaster_data": [],
                    "account_deployment_data": [],
                    "contract_class": {
                        "sierra_program": ["0xAA", "0xBB"],
                        "contract_class_version": "1.0",
                        "entry_points_by_type": {
                            "EXTERNAL": [
                                {
                                    "selector": "0x3c118a68e16e12e97ed25cb4901c12f4d3162818669cc44c391d8049924c14",
                                    "function_idx": 4
                                },
                                {
                                    "selector": "0xe7510edcf6e9f1b70f7bd1f488767b50f0363422f3c563160ab77adf62467b",
                                    "function_idx": 7
                                }
                            ],
                            "L1_HANDLER": [
                                {
                                    "selector": "0x39edbbb129ad752107a94d40c3873cae369a46fd2fc578d075679aa67e85d12",
                                    "function_idx": 11
                                }
                            ],
                            "CONSTRUCTOR": [
                                {
                                    "selector": "0x28ffe4ff0f226a9107253e17a904099aa4f63a02a5621de0576e5aa71bc5194",
                                    "function_idx": 12
                                }
                            ]
                        },
                        "abi": [
                            {
                                "type": "constructor",
                                "name": "constructor",
                                "inputs": [
                                    {
                                        "name": "arg1",
                                        "type": "core::felt252"
                                    },
                                    {
                                        "name": "arg2",
                                        "type": "core::felt252"
                                    }
                                ]
                            }
                        ]
                    },
                    "nonce_data_availability_mode": "L1",
                    "fee_data_availability_mode": "L1"
                },
                {
                    "type": "INVOKE",
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
                    "version": "0x100000000000000000000000000000003",
                    "signature": ["0x2"],
                    "nonce": "0x1",
                    "sender_address": "0x3",
                    "calldata": [
                        "0x1",
                        "0x2",
                        "0x3"
                    ],
                    "nonce_data_availability_mode": "L1",
                    "fee_data_availability_mode": "L1"
                },
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
            ],
            "block_id": {
                "block_number": 1
            },
            "simulation_flags": []
        }"#;

        let estimate_fee_input = serde_json::from_str::<super::EstimateFeeInput>(json_str).unwrap();
        assert_eq!(estimate_fee_input.block_id, BlockId::Number(1));
        assert_eq!(estimate_fee_input.request.len(), 3);
        assert!(matches!(
            estimate_fee_input.request[0],
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(_))
        ));
        assert!(matches!(estimate_fee_input.request[1], BroadcastedTransaction::Invoke(_)));
        assert!(matches!(estimate_fee_input.request[2], BroadcastedTransaction::DeployAccount(_)));
    }

    #[test]
    fn deserialize_call_input() {
        let json_str = r#"{"request": {"contract_address": "0x01", "entry_point_selector": "0x02", "calldata": ["0x03"]}, "block_id": {"block_number": 1}}"#;
        let call_input = serde_json::from_str::<super::CallInput>(json_str).unwrap();

        assert_eq!(
            call_input,
            super::CallInput {
                request: super::FunctionCall {
                    contract_address: ContractAddress::new(Felt::ONE).unwrap(),
                    entry_point_selector: Felt::TWO,
                    calldata: vec![Felt::THREE],
                },
                block_id: BlockId::Number(1),
            }
        );
    }

    #[test]
    fn deserialize_get_storage_input() {
        fn assert_get_storage_input_correctness(
            should_be_correct: bool,
            expected_storage_input: GetStorageInput,
            json_str: &str,
        ) {
            let is_correct =
                if let Ok(get_storage_input) = serde_json::from_str::<GetStorageInput>(json_str) {
                    get_storage_input == expected_storage_input
                } else {
                    false
                };

            assert_eq!(should_be_correct, is_correct);
        }

        let expected_storage_input = GetStorageInput {
            block_id: BlockId::Hash(Felt::ONE),
            contract_address: ContractAddress::new(Felt::TWO).unwrap(),
            key: PatriciaKey::new(Felt::THREE).unwrap(),
        };

        assert_get_storage_input_correctness(
            true,
            expected_storage_input.clone(),
            r#"{"block_id": {"block_hash": "0x01"}, "contract_address": "0x02", "key": "0x03"}"#,
        );

        // Incorrect contract_address key
        assert_get_storage_input_correctness(
            false,
            expected_storage_input.clone(),
            r#"{"block_id": {"block_hash": "0x01"}, "contract_address_mock": "0x02", "key": "0x03"}"#,
        );

        // Incorrect key
        assert_get_storage_input_correctness(
            false,
            expected_storage_input,
            r#"{"block_id": {"block_hash": "0x01"}, "contract_address": "0x02", "keyy": "0x03"}"#,
        );
    }

    // unit tests for TransactionHashInput deserialization
    #[test]
    fn deserialize_transaction_hash_input() {
        assert_transaction_hash_correctness(true, "0x01", r#"{"transaction_hash": "0x01"}"#);

        // Incorrect transaction_hash key
        assert_transaction_hash_correctness(false, "0x01", r#"{"transaction_hashh": "0x01"}"#);

        // Incorrect transaction_hash value
        assert_transaction_hash_correctness(false, "0x02", r#"{"transaction_hash": "0x01"}"#);

        // Incorrect transaction_hash format, should be prefixed with 0x
        assert_transaction_hash_correctness(false, "0x02", r#"{"transaction_hash": "01"}"#);
    }
    #[test]
    fn deserialize_block_id_tag_variants() {
        assert_block_id_tag_correctness(true, BlockTag::Latest, r#"{"block_id": "latest"}"#);
        assert_block_id_tag_correctness(
            true,
            BlockTag::PreConfirmed,
            r#"{"block_id": "pre_confirmed"}"#,
        );

        // Incorrect tag
        assert_block_id_tag_correctness(false, BlockTag::Latest, r#"{"block_id": "latestx"}"#);
        assert_block_id_tag_correctness(false, BlockTag::Latest, r#"{"block_id": "pending"}"#);
        assert_block_id_tag_correctness(
            false,
            BlockTag::PreConfirmed,
            r#"{"block_id": "pre_confirmed_d"}"#,
        );

        // Incorrect key
        assert_block_id_tag_correctness(false, BlockTag::Latest, r#"{"block": "latest"}"#);
        assert_block_id_tag_correctness(false, BlockTag::PreConfirmed, r#"{"block": "pending"}"#);
        assert_block_id_tag_correctness(
            false,
            BlockTag::PreConfirmed,
            r#"{"block": "pre_confirmed"}"#,
        );
    }

    #[test]
    fn deserialize_block_id_block_hash_variants() {
        assert_block_id_block_hash_correctness(
            true,
            "0x01",
            r#"{"block_id": {"block_hash": "0x01"}}"#,
        );

        // BlockId's key is block instead of block_id
        assert_block_id_block_hash_correctness(
            false,
            "0x01",
            r#"{"block": {"block_hash": "0x01"}}"#,
        );

        // Incorrect block_hash key
        assert_block_id_block_hash_correctness(
            false,
            "0x01",
            r#"{"block_id": {"block_hasha": "0x01"}}"#,
        );

        // Incorrect block_hash value
        assert_block_id_block_hash_correctness(
            false,
            "0x02",
            r#"{"block_id": {"block_hash": "0x01"}}"#,
        );

        // Block hash hex value is more than 64 chars
        assert_block_id_block_hash_correctness(
            false,
            "0x01",
            r#"{"block_id": {"block_hash": "0x004134134134134134134134134134134134134134134134134134134134134134"}}"#,
        );

        // Block hash hex doesn't start with 0x
        assert_block_id_block_hash_correctness(
            false,
            "0x01",
            r#"{"block_id": {"block_hash": "01"}}"#,
        );
    }

    #[test]
    fn deserialize_block_id_block_number_variants() {
        assert_block_id_block_number_correctness(true, 10, r#"{"block_id": {"block_number": 10}}"#);

        // BlockId's key is block instead of block_id
        assert_block_id_block_number_correctness(false, 10, r#"{"block": {"block_number": 10}}"#);

        // Incorrect block_number key
        assert_block_id_block_number_correctness(
            false,
            10,
            r#"{"block_id": {"block_number_mock": 10}}"#,
        );

        // Incorrect block_number value
        assert_block_id_block_number_correctness(
            false,
            10,
            r#"{"block_id": {"block_number": "0x01"}}"#,
        );
    }

    #[test]
    fn assert_error_message_for_failed_block_id_deserialization() {
        for json_str in [
            r#"{"block_id": {"block_number": 10, "block_hash": "0x1"}}"#,
            r#"{"block_id": {"block_number": "123"}}"#,
            r#"{"block_id": {"block_number": -123}}"#,
            r#"{"block_id": {"invalid_key": ""}}"#,
            r#"{"block_id": {"block_hash": 123}}"#,
            r#"{"block_id": {"block_hash": ""}}"#,
        ] {
            match serde_json::from_str::<BlockIdInput>(json_str) {
                Err(e) => assert_contains(&e.to_string(), EXPECTED_INVALID_BLOCK_ID_MSG).unwrap(),
                other => panic!("Invalid result: {other:?}"),
            }
        }
    }

    fn assert_block_id_tag_correctness(
        should_be_correct: bool,
        expected_tag: BlockTag,
        json_str_block_id: &str,
    ) {
        let is_correct =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
                .map(|BlockIdInput { block_id }| matches!(block_id, BlockId::Tag(generated_tag) if generated_tag == expected_tag))
                .unwrap_or(false);

        assert_eq!(should_be_correct, is_correct);
    }

    fn assert_block_id_block_number_correctness(
        should_be_correct: bool,
        expected_block_number: u64,
        json_str_block_id: &str,
    ) {
        let is_correct =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
                .map(
                    |BlockIdInput { block_id }|
                    matches!(block_id,
                    BlockId::Number(generated_block_number) if generated_block_number == expected_block_number)
            ).unwrap_or(false);

        assert_eq!(should_be_correct, is_correct);
    }

    fn assert_block_id_block_hash_correctness(
        should_be_correct: bool,
        expected_block_hash: &str,
        json_str_block_id: &str,
    ) {
        let is_correct =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
                .map(|BlockIdInput { block_id }| matches!(block_id, BlockId::Hash(generated_block_hash) if generated_block_hash == felt_from_prefixed_hex(expected_block_hash).unwrap()))
        .unwrap_or(false);

        assert_eq!(should_be_correct, is_correct)
    }

    fn assert_transaction_hash_correctness(
        should_be_correct: bool,
        expected_transaction_hash: &str,
        json_str_transaction_hash: &str,
    ) {
        let is_correct = if let Ok(transaction_hash_input) =
            serde_json::from_str::<super::TransactionHashInput>(json_str_transaction_hash)
        {
            transaction_hash_input.transaction_hash
                == felt_from_prefixed_hex(expected_transaction_hash).unwrap()
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct);
    }
}
