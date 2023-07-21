pub mod common;

mod get_transaction_by_hash_integration_tests {
    use starknet_core::constants::DECLARE_V1_TRANSACTION_HASH;
    use starknet_rs_core::types::{BroadcastedDeclareTransactionV1, FieldElement, contract::SierraClass};
    use starknet_rs_providers::Provider;
    use crate::common::util::BackgroundDevnet;

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
    async fn get_declere_v2_transaction_by_hash_happy_path() {

        // Sierra class artifact. Output of the `starknet-compile` command
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/declare_v2_contract/output.json");
        let contract_artifact: SierraClass = serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        
        println!("contract_class_version: {}", contract_artifact.contract_class_version);
    }
}
