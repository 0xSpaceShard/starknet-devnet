use serde::{Deserialize, Serialize};
use starknet_types::starknet_api::block::BlockNumber;

use super::{
    block::{BlockHashHex, SyncStatus},
    transaction::{ClassHashHex, EventFilter, FunctionCall, Transaction, TransactionHashHex},
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
    pub request: Vec<Transaction>,
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
    use starknet_types::{felt::Felt, starknet_api::block::BlockNumber};

    use crate::api::json_rpc::models::{BlockHashOrNumber, BlockId, FeltHex, Tag};

    use super::BlockIdInput;

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
        assert_block_id_block_number_correctness(
            true,
            10,
            r#"{"block_id": {"block_number": 10}}"#,
        );

        // BlockId's key is block instead of block_id
        assert_block_id_block_number_correctness(
            false,
            10,
            r#"{"block": {"block_number": 10}}"#,
        );

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
                BlockId::HashOrNumber(hash_or_number) => match hash_or_number {
                    BlockHashOrNumber::Hash(generated_block_hash) => {
                        generated_block_hash
                            == FeltHex(Felt::from_prefixed_hex_str(expected_block_hash).unwrap())
                    }
                    _ => false,
                },
                _ => false,
            }
        } else {
            false
        };

        assert_eq!(should_be_correct, is_correct)
    }
}
