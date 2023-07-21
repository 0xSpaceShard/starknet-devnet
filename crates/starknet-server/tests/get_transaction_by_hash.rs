pub mod common;

mod get_transaction_by_hash_integration_tests {
    use starknet_core::constants::DECLARE_V1_TRANSACTION_HASH;
    use starknet_rs_core::types::{BroadcastedDeclareTransactionV1, FieldElement, contract::{SierraClass, CompiledClass}};
    use starknet_rs_providers::Provider;
    use starknet_types::{felt::Felt, traits::ToHexString};
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
        let path_to_cario1 = concat!(env!("CARGO_MANIFEST_DIR"), r"\test_data\rpc\declare_v2_contract_cario1\output.json");
        let contract_artifact: SierraClass = serde_json::from_reader(std::fs::File::open(path_to_cario1).unwrap()).unwrap();

        let path_to_casm = concat!(env!("CARGO_MANIFEST_DIR"), r"\test_data\rpc\declare_v2_contract_cario1\output-casm.json");
        let casm_contract_definition: CompiledClass =  serde_json::from_reader(std::fs::File::open(path_to_casm).unwrap()).unwrap();
        let compiled_class_hash: Result<FieldElement, starknet_rs_core::types::contract::ComputeClassHashError> = casm_contract_definition.class_hash();
        assert_eq!(Felt::from(compiled_class_hash.unwrap()).to_prefixed_hex_str(), "0x63b33a5f2f46b1445d04c06d7832c48c48ad087ce0803b71f2b8d96353716ca");
    }
}
