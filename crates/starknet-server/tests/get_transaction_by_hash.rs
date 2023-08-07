pub mod common;

mod get_transaction_by_hash_integration_tests {
    use std::sync::Arc;

    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::{CompiledClass, SierraClass};
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, FieldElement,
    };
    use starknet_rs_providers::Provider;
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;
    use starknet_types::traits::ToHexString;

    use crate::common::constants::{
        CASM_COMPILED_CLASS_HASH, PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    };
    use crate::common::util::BackgroundDevnet;

    pub const DECLARE_V1_TRANSACTION_HASH: &str =
        "0x01d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627";

    pub const DECLARE_V2_TRANSACTION_HASH: &str =
        "0x040b80108251e5991622eb2ff6061313dabe66a52f550c59867c027910777e7e";

    #[tokio::test]
    async fn get_declare_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await
            .unwrap();

        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(declare_transaction.transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::Declare(
            starknet_rs_core::types::DeclareTransaction::V1(declare_v1),
        ) = result
        {
            assert_eq!(
                declare_v1.transaction_hash,
                FieldElement::from_hex_be(DECLARE_V1_TRANSACTION_HASH).unwrap()
            );
        } else {
            panic!();
        }
    }

    #[tokio::test]
    async fn get_declare_v2_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // Sierra class artifact. Output of the `starknet-compile` command.
        let path_to_cairo1 =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/contract_cairo_v1/output.json");
        let contract_artifact: SierraClass =
            serde_json::from_reader(std::fs::File::open(path_to_cairo1).unwrap()).unwrap();

        // Casm artifact. Output of the `starknet-sierra-compile` command.
        let path_to_casm = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/contract_cairo_v1/output-casm.json"
        );
        let casm_contract_definition: CompiledClass =
            serde_json::from_reader(std::fs::File::open(path_to_casm).unwrap()).unwrap();
        let compiled_class_hash = (casm_contract_definition.class_hash()).unwrap();
        assert_eq!(Felt::from(compiled_class_hash).to_prefixed_hex_str(), CASM_COMPILED_CLASS_HASH);

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_PRIVATE_KEY).unwrap(),
        ));
        let address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let mut account =
            SingleOwnerAccount::new(&devnet.json_rpc_client, signer, address, chain_id::TESTNET);
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        // We need to flatten the ABI into a string first
        let flattened_class = contract_artifact.flatten().unwrap();
        let result = account
            .declare(Arc::new(flattened_class), compiled_class_hash)
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from_hex_be("0xde0b6b3a7640000").unwrap()) // Specified max fee of 10^18 to declare v2 transaction, can be removed once fee estimation will work
            .send()
            .await;

        assert_eq!(
            result.unwrap().transaction_hash,
            FieldElement::from_hex_be(DECLARE_V2_TRANSACTION_HASH).unwrap()
        );
    }
}
