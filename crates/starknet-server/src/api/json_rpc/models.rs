use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{BlockHash, ClassHash, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{BlockId, SyncStatus};
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedTransaction, EventFilter, FunctionCall,
    SimulationFlag,
};
use starknet_types::starknet_api::block::BlockNumber;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockIdInput {
    pub(crate) block_id: BlockId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionHashInput {
    pub(crate) transaction_hash: TransactionHash,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GetStorageInput {
    pub(crate) contract_address: ContractAddress,
    pub(crate) key: PatriciaKey,
    pub(crate) block_id: BlockId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockAndIndexInput {
    pub(crate) block_id: BlockId,
    pub(crate) index: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockAndClassHashInput {
    pub(crate) block_id: BlockId,
    pub(crate) class_hash: ClassHash,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockAndContractAddressInput {
    pub(crate) block_id: BlockId,
    pub(crate) contract_address: ContractAddress,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallInput {
    pub request: FunctionCall,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateFeeInput {
    pub request: Vec<BroadcastedTransaction>,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockHashAndNumberOutput {
    pub block_hash: BlockHash,
    pub block_number: BlockNumber,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SyncingOutput {
    True(SyncStatus),
    False(bool),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventsInput {
    pub filter: EventFilter,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum BroadcastedDeclareTransactionEnumWrapper {
    #[serde(rename = "DECLARE")]
    Declare(BroadcastedDeclareTransaction),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionInput {
    pub declare_transaction: BroadcastedDeclareTransactionEnumWrapper,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionOutput {
    pub transaction_hash: TransactionHash,
    pub class_hash: ClassHash,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum BroadcastedDeployAccountTransactionEnumWrapper {
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(BroadcastedDeployAccountTransaction),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionInput {
    pub deploy_account_transaction: BroadcastedDeployAccountTransactionEnumWrapper,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeployAccountTransactionOutput {
    pub transaction_hash: TransactionHash,
    pub contract_address: ContractAddress,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum BroadcastedInvokeTransactionEnumWrapper {
    #[serde(rename = "INVOKE")]
    Invoke(BroadcastedInvokeTransaction),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionInput {
    pub invoke_transaction: BroadcastedInvokeTransactionEnumWrapper,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeTransactionOutput {
    pub transaction_hash: TransactionHash,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulateTransactionsInput {
    pub block_id: BlockId,
    pub transactions: Vec<BroadcastedTransaction>,
    pub simulation_flags: Vec<SimulationFlag>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionStatusOutput {
    pub finality_status: TransactionFinalityStatus,
    pub execution_status: TransactionExecutionStatus,
}

#[cfg(test)]
mod tests {
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::patricia_key::PatriciaKey;
    use starknet_types::rpc::block::{BlockHashOrNumber, BlockId, Tag};
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, BroadcastedTransaction,
    };
    use starknet_types::starknet_api::block::BlockNumber;

    use super::{BlockIdInput, EstimateFeeInput, GetStorageInput};
    use crate::api::json_rpc::requests_tests::assert_contains;

    #[test]
    fn errored_deserialization_of_estimate_fee_with_broadcasted_declare_transaction() {
        // Errored json struct that passed DECLARE V2, but contract class is of type V1
        let json_str = r#"{
            "request": [{
                "type": "DECLARE",
                "max_fee": "0xA",
                "version": "0x2",
                "signature": ["0xFF", "0xAA"],
                "nonce": "0x0",
                "sender_address": "0x0001",
                "compiled_class_hash": "0x01",
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
                }
            }],
            "block_id": {
                "block_number": 1
            }
        }"#;

        match serde_json::from_str::<EstimateFeeInput>(json_str) {
            Err(err) => assert_contains(&err.to_string(), "Invalid declare transaction v2"),
            other => panic!("Invalid result: {other:?}"),
        }
    }

    #[test]
    fn deserialize_estimate_fee_input() {
        let json_str = r#"{
            "request": [
                {
                    "type": "DECLARE",
                    "max_fee": "0xA", 
                    "version": "0x1", 
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "sender_address": "0x0001",
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
                        "entry_points_by_type": {
                            "CONSTRUCTOR": [], 
                            "EXTERNAL": [], 
                            "L1_HANDLER": []
                        }
                    }
                },
                {
                    "type": "DECLARE",
                    "max_fee": "0xA",
                    "version": "0x2",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "sender_address": "0x0001",
                    "compiled_class_hash": "0x01",
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
                    }
                },
                {
                    "type": "INVOKE",
                    "max_fee": "0x1",
                    "version": "0x100000000000000000000000000000001",
                    "signature": [
                    "0x2"
                    ],
                    "nonce": "0x1",
                    "sender_address": "0x3",
                    "calldata": [
                    "0x1",
                    "0x2",
                    "0x3"
                  ]
                },
                {
                    "type": "DEPLOY_ACCOUNT",
                    "max_fee": "0xA",
                    "version": "0x1",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "contract_address_salt": "0x01",
                    "constructor_calldata": ["0x01"],
                    "class_hash": "0x01"
                }
                ], 
            "block_id": {
                "block_number": 1
            }
        }"#;

        let estimate_fee_input = serde_json::from_str::<super::EstimateFeeInput>(json_str).unwrap();
        assert_eq!(
            estimate_fee_input.block_id,
            BlockId::HashOrNumber(BlockHashOrNumber::Number(BlockNumber(1)))
        );
        assert_eq!(estimate_fee_input.request.len(), 4);
        assert!(matches!(
            estimate_fee_input.request[0],
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(_))
        ));
        assert!(matches!(
            estimate_fee_input.request[1],
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(_))
        ));
        assert!(matches!(estimate_fee_input.request[2], BroadcastedTransaction::Invoke(_)));
        assert!(matches!(estimate_fee_input.request[3], BroadcastedTransaction::DeployAccount(_)));
    }

    #[test]
    fn deserialize_call_input() {
        let json_str = r#"{"request": {"contract_address": "0x01", "entry_point_selector": "0x02", "calldata": ["0x03"]}, "block_id": {"block_number": 1}}"#;
        let call_input = serde_json::from_str::<super::CallInput>(json_str).unwrap();

        assert_eq!(
            call_input,
            super::CallInput {
                request: super::FunctionCall {
                    contract_address: ContractAddress::new(
                        Felt::from_prefixed_hex_str("0x01").unwrap()
                    )
                    .unwrap(),
                    entry_point_selector: Felt::from_prefixed_hex_str("0x02").unwrap(),
                    calldata: vec![Felt::from_prefixed_hex_str("0x03").unwrap()],
                },
                block_id: BlockId::HashOrNumber(BlockHashOrNumber::Number(BlockNumber(1))),
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
            block_id: BlockId::HashOrNumber(BlockHashOrNumber::Hash(
                Felt::from_prefixed_hex_str("0x01").unwrap(),
            )),
            contract_address: ContractAddress::new(Felt::from_prefixed_hex_str("0x02").unwrap())
                .unwrap(),
            key: PatriciaKey::new(Felt::from_prefixed_hex_str("0x03").unwrap()).unwrap(),
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
            r#"{"block_id": {"block_hash": "0x01"}, "contract_addresss": "0x02", "key": "0x03"}"#,
        );

        // Incorrect key key
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
        assert_block_id_tag_correctness(true, Tag::Latest, r#"{"block_id": "latest"}"#);
        assert_block_id_tag_correctness(true, Tag::Pending, r#"{"block_id": "pending"}"#);

        // Incorrect tag
        assert_block_id_tag_correctness(false, Tag::Latest, r#"{"block_id": "latests"}"#);
        assert_block_id_tag_correctness(false, Tag::Pending, r#"{"block_id": "pendingg"}"#);

        // Incorrect key
        assert_block_id_tag_correctness(false, Tag::Latest, r#"{"block": "latest"}"#);
        assert_block_id_tag_correctness(false, Tag::Pending, r#"{"block": "pending"}"#);
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

        // Block hash hex doesnt start with 0x
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
            r#"{"block_id": {"block_numberr": 10}}"#,
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
        for (json_str, expected_msg) in [
            (
                r#"{"block_id": {"block_number": 10, "block_hash": "0x1"}}"#,
                "expected map with a single key",
            ),
            (
                r#"{"block_id": {"block_number": "123"}}"#,
                "Invalid block ID: invalid type: string \"123\", expected u64",
            ),
            (r#"{"block_id": {"block_number": -123}}"#, "Invalid block ID: invalid number"),
            (
                r#"{"block_id": {"invalid_key": ""}}"#,
                "Invalid block ID: unknown variant `invalid_key`, expected `block_hash` or \
                 `block_number`",
            ),
            (
                r#"{"block_id": {"block_hash": 123}}"#,
                "Invalid block ID: invalid type: number, expected a string",
            ),
            (r#"{"block_id": {"block_hash": ""}}"#, "Invalid block ID: Missing prefix 0x"),
        ] {
            match serde_json::from_str::<BlockIdInput>(json_str) {
                Err(err) => assert_contains(&err.to_string(), expected_msg),
                other => panic!("Invalid result: {other:?}"),
            }
        }
    }

    fn assert_block_id_tag_correctness(
        should_be_correct: bool,
        expected_tag: Tag,
        json_str_block_id: &str,
    ) {
        let is_correct = if let Ok(BlockIdInput { block_id: BlockId::Tag(generated_tag) }) =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            generated_tag == expected_tag
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct);
    }

    fn assert_block_id_block_number_correctness(
        should_be_correct: bool,
        expected_block_number: u64,
        json_str_block_id: &str,
    ) {
        let is_correct = if let Ok(BlockIdInput {
            block_id: BlockId::HashOrNumber(BlockHashOrNumber::Number(generated_block_number)),
        }) = serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            generated_block_number == BlockNumber(expected_block_number)
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct);
    }

    fn assert_block_id_block_hash_correctness(
        should_be_correct: bool,
        expected_block_hash: &str,
        json_str_block_id: &str,
    ) {
        let is_correct = if let Ok(BlockIdInput {
            block_id: BlockId::HashOrNumber(BlockHashOrNumber::Hash(generated_block_hash)),
        }) = serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            generated_block_hash == Felt::from_prefixed_hex_str(expected_block_hash).unwrap()
        } else {
            false
        };

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
                == Felt::from_prefixed_hex_str(expected_transaction_hash).unwrap()
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct);
    }
}
