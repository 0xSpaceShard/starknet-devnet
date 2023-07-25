pub mod common;

mod get_transaction_by_hash_integration_tests {
    use std::sync::Arc;
    use starknet_core::constants::DECLARE_V1_TRANSACTION_HASH;
    use starknet_rs_core::{chain_id, types::{BroadcastedDeclareTransactionV1, BroadcastedDeclareTransactionV2, FieldElement, BlockId, BlockTag, contract::{SierraClass, CompiledClass}}};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_providers::Provider;
    use starknet_types::{felt::Felt, traits::ToHexString};
    use crate::common::util::BackgroundDevnet;
    use crate::common::constants::{PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_PRIVATE_KEY};

    #[tokio::test]
    async fn get_declere_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let add_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(
                starknet_rs_core::types::BroadcastedDeclareTransaction::V1(declare_txn_v1.clone()),
            )
            .await
            .unwrap();

        let get_transaction = devnet
            .json_rpc_client
            .get_transaction_by_hash(add_transaction.transaction_hash)
            .await
            .unwrap();

        match get_transaction.clone() {
            starknet_rs_core::types::Transaction::Declare(starknet_rs_core::types::DeclareTransaction::V1(declare_v1)) => {
                assert_eq!(declare_v1.transaction_hash, FieldElement::from_hex_be(DECLARE_V1_TRANSACTION_HASH).unwrap());
            },
            _ => {}
        };
    }


    #[tokio::test]
    async fn get_declere_v123_transaction_by_hash_happy_path() {

        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v2.json"
        )).unwrap();

        let declare_txn_v2: BroadcastedDeclareTransactionV2 =
            serde_json::from_str(&json_string).unwrap();

        let add_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(
                starknet_rs_core::types::BroadcastedDeclareTransaction::V2(declare_txn_v2.clone()),
            )
            .await
            .unwrap();

        println!("result.transaction_hash: {}", Felt::from(add_transaction.transaction_hash).to_prefixed_hex_str());

        let get_transaction = devnet
            .json_rpc_client
            .get_transaction_by_hash(add_transaction.transaction_hash)
            .await
            .unwrap();

        match get_transaction.clone() {
            starknet_rs_core::types::Transaction::Declare(starknet_rs_core::types::DeclareTransaction::V1(declare_v1)) => {
                assert_eq!(declare_v1.transaction_hash, FieldElement::from_hex_be(DECLARE_V1_TRANSACTION_HASH).unwrap());
            },
            _ => {}
        };
    }


    #[tokio::test]
    async fn get_declere_v2_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // Sierra class artifact. Output of the `starknet-compile` command
        let path_to_cario1 = concat!(env!("CARGO_MANIFEST_DIR"), r"\test_data\rpc\declare_v2_contract_cario1\output.json");
        let contract_artifact: SierraClass = serde_json::from_reader(std::fs::File::open(path_to_cario1).unwrap()).unwrap();

        let path_to_casm = concat!(env!("CARGO_MANIFEST_DIR"), r"\test_data\rpc\declare_v2_contract_cario1\output-casm.json");
        let casm_contract_definition: CompiledClass =  serde_json::from_reader(std::fs::File::open(path_to_casm).unwrap()).unwrap();
        let compiled_class_hash = (casm_contract_definition.class_hash()).unwrap();
        assert_eq!(Felt::from(compiled_class_hash).to_prefixed_hex_str(), "0x63b33a5f2f46b1445d04c06d7832c48c48ad087ce0803b71f2b8d96353716ca");

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_PRIVATE_KEY).unwrap(),
        ));
        let address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let mut account = SingleOwnerAccount::new(&devnet.json_rpc_client, signer, address, chain_id::TESTNET);
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        // We need to flatten the ABI into a string first
        let flattened_class = contract_artifact.flatten().unwrap();

        let result = account
            .declare(Arc::new(flattened_class), compiled_class_hash)
            .nonce(FieldElement::from_hex_be("0x0").unwrap())
            .max_fee(FieldElement::from_hex_be("0xde0b6b3a7640000").unwrap())
            .prepared()
            .unwrap()
            .send()
            .await;

        println!("result.transaction_hash: {}", Felt::from(result.unwrap().transaction_hash).to_prefixed_hex_str());
    
    }
}
