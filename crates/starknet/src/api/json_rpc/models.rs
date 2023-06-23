use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::block::BlockNumber;

use crate::api::models::{
    block::{BlockHashHex, SyncStatus},
    transaction::{
        BroadcastedTransactionWithType, ClassHashHex, EventFilter, FunctionCall, TransactionHashHex,
    },
    BlockId, ContractAddressHex, PatriciaKeyHex,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockIdInput {
    pub(crate) block_id: BlockId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TransactionHashInput {
    pub(crate) transaction_hash: TransactionHashHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct GetStorageInput {
    pub(crate) block_id: BlockId,
    pub(crate) contract_address: ContractAddressHex,
    pub(crate) key: PatriciaKeyHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndIndexInput {
    pub(crate) block_id: BlockId,
    pub(crate) index: BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndClassHashInput {
    pub(crate) block_id: BlockId,
    pub(crate) class_hash: ClassHashHex,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BlockAndContractAddressInput {
    pub(crate) block_id: BlockId,
    pub(crate) contract_address: ContractAddressHex,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct CallInput {
    pub request: FunctionCall,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EstimateFeeInput {
    pub request: Vec<BroadcastedTransactionWithType>,
    pub block_id: BlockId,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EstimateFeeOutput {
    pub gas_consumed: String,
    pub gas_price: String,
    pub overall_fee: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockHashAndNumberOutput {
    pub block_hash: BlockHashHex,
    pub block_number: BlockNumber,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum SyncingOutput {
    True(SyncStatus),
    False(bool),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventsInput {
    pub filter: EventFilter,
}

#[cfg(test)]
mod tests {
    use starknet_types::{
        contract_address::ContractAddress, felt::Felt, patricia_key::PatriciaKey,
        starknet_api::block::BlockNumber,
    };

    use super::{EstimateFeeInput, GetStorageInput};
    use crate::api::models::{
        transaction::{
            BroadcastedDeclareTransaction, BroadcastedInvokeTransaction, BroadcastedTransaction,
        },
        BlockHashOrNumber, BlockId, ContractAddressHex, FeltHex, PatriciaKeyHex, Tag,
    };

    use super::BlockIdInput;

    #[test]
    fn errored_deserialization_of_estimate_fee_with_broadcasted_declare_transaction() {
        // Errored json struct that passed DECLARE V2, but contract class is of type V1
        let json_str = r#"{
            "request": [
                "type": "DECLARE",
                "max_fee": "0xA",
                "version": "0x1",
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
            ],
            "block_id": {
                "block_number": 1
            }
        }"#;

        assert!(serde_json::from_str::<EstimateFeeInput>(json_str).is_err());
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
                        "entry_points_by_type": {}
                    }
                },
                {
                    "type": "DECLARE",
                    "max_fee": "0xA",
                    "version": "0x1",
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
                                    "selector": "0xAAE3B5E8",
                                    "function_idx": 1
                                },
                                {
                                    "selector": "0xAAE3B5E9",
                                    "function_idx": 2
                                }
                            ]
                        },
                        "abi": "H4sIAAAAAAAA/8tIzcnJVyjPL8pJUQQAlQYXAAAA"
                    }
                },
                {
                    "type": "INVOKE",
                    "max_fee": "0xA",
                    "version": "0x1",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "contract_address": "0x0001",
                    "entry_point_selector": "0x01",
                    "calldata": ["0x01"]
                }, 
                {
                    "type": "INVOKE",
                    "max_fee": "0xA",
                    "version": "0x1",
                    "signature": ["0xFF", "0xAA"],
                    "nonce": "0x0",
                    "sender_address": "0x0001",
                    "calldata": ["0x01"]
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
        assert!(
            estimate_fee_input.block_id
                == BlockId::HashOrNumber(BlockHashOrNumber::Number(BlockNumber(1)))
        );
        assert!(estimate_fee_input.request.len() == 5);
        assert!(matches!(
            estimate_fee_input.request[0].transaction,
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(_))
        ));
        assert!(matches!(
            estimate_fee_input.request[1].transaction,
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(_))
        ));
        assert!(matches!(
            estimate_fee_input.request[2].transaction,
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(_))
        ));
        assert!(matches!(
            estimate_fee_input.request[3].transaction,
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(_))
        ));
        assert!(matches!(
            estimate_fee_input.request[4].transaction,
            BroadcastedTransaction::DeployAccount(_)
        ));
    }

    #[test]
    fn deserialize_call_input() {
        let json_str = r#"{"request": {"contract_address": "0x01", "entry_point_selector": "0x02", "calldata": ["0x03"]}, "block_id": {"block_number": 1}}"#;
        let call_input = serde_json::from_str::<super::CallInput>(json_str).unwrap();

        assert_eq!(
            call_input,
            super::CallInput {
                request: super::FunctionCall {
                    contract_address: ContractAddressHex(
                        ContractAddress::new(Felt::from_prefixed_hex_str("0x01").unwrap()).unwrap()
                    ),
                    entry_point_selector: FeltHex(Felt::from_prefixed_hex_str("0x02").unwrap()),
                    calldata: vec![FeltHex(Felt::from_prefixed_hex_str("0x03").unwrap())],
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
            block_id: BlockId::HashOrNumber(BlockHashOrNumber::Hash(FeltHex(
                Felt::from_prefixed_hex_str("0x01").unwrap(),
            ))),
            contract_address: ContractAddressHex(
                ContractAddress::new(Felt::from_prefixed_hex_str("0x02").unwrap()).unwrap(),
            ),
            key: PatriciaKeyHex(
                PatriciaKey::new(Felt::from_prefixed_hex_str("0x03").unwrap()).unwrap(),
            ),
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

    fn assert_block_id_tag_correctness(
        should_be_correct: bool,
        expected_tag: Tag,
        json_str_block_id: &str,
    ) {
        let is_correct = if let Ok(BlockIdInput { block_id }) =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            match block_id {
                BlockId::Tag(generated_tag) => generated_tag == expected_tag,
                _ => false,
            }
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
        let is_correct = if let Ok(BlockIdInput { block_id }) =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            match block_id {
                BlockId::HashOrNumber(hash_or_number) => match hash_or_number {
                    BlockHashOrNumber::Number(generated_block_number) => {
                        generated_block_number == BlockNumber(expected_block_number)
                    }
                    _ => false,
                },
                _ => false,
            }
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
        let is_correct = if let Ok(BlockIdInput { block_id }) =
            serde_json::from_str::<BlockIdInput>(json_str_block_id)
        {
            match block_id {
                BlockId::HashOrNumber(BlockHashOrNumber::Hash(generated_block_hash)) => {
                    generated_block_hash
                        == FeltHex(Felt::from_prefixed_hex_str(expected_block_hash).unwrap())
                }
                _ => false,
            }
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
                == FeltHex(Felt::from_prefixed_hex_str(expected_transaction_hash).unwrap())
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct);
    }
}
