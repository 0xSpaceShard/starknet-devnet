pub mod common;

mod get_transaction_by_hash_integration_tests {
    use std::sync::Arc;

    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::{CompiledClass, SierraClass};
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, BroadcastedDeployAccountTransaction,
        BroadcastedInvokeTransactionV1, FieldElement, StarknetError,
    };
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;
    use starknet_types::traits::ToHexString;

    use crate::common::constants::{
        CASM_COMPILED_CLASS_HASH, PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    };
    use crate::common::devnet::BackgroundDevnet;

    pub const DECLARE_V1_TRANSACTION_HASH: &str =
        "0x01d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627";

    pub const DECLARE_V2_TRANSACTION_HASH: &str =
        "0x040b80108251e5991622eb2ff6061313dabe66a52f550c59867c027910777e7e";

    pub const DEPLOY_ACCOUNT_TRANSACTION_HASH: &str =
        "0x01b815e18639e72c8f56f5dbee70c5997f127e6183e46ea26a41b7f2944be593";

    pub const INVOKE_V1_TRANSACTION_HASH: &str =
        "0x057c60c720f9ce34cd0a411e5c2ded91dfd2a912c11a26508c796da53d1b73c6";

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
            panic!("Could not unpack the transaction from {result:?}");
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
        let declare_result = account
            .declare(Arc::new(flattened_class), compiled_class_hash)
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from_hex_be("0xde0b6b3a7640000").unwrap()) // Specified max fee of 10^18 to declare v2 transaction, can be removed once fee estimation will work
            .send()
            .await;

        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(declare_result.unwrap().transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::Declare(
            starknet_rs_core::types::DeclareTransaction::V2(declare_v2),
        ) = result
        {
            assert_eq!(
                declare_v2.transaction_hash,
                FieldElement::from_hex_be(DECLARE_V2_TRANSACTION_HASH).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {result:?}");
        }
    }

    #[tokio::test]
    async fn get_deploy_account_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let deploy_account_txn = BroadcastedDeployAccountTransaction {
            max_fee: FieldElement::from_hex_be("0xde0b6b3a7640000").unwrap(),
            signature: vec![
                FieldElement::from_hex_be("0x1").unwrap(),
                FieldElement::from_hex_be("0x1").unwrap(),
            ],
            nonce: FieldElement::from_hex_be("0x0").unwrap(),
            class_hash: FieldElement::from_hex_be(
                "0x7B3E05F48F0C69E4A65CE5E076A66271A527AFF2C34CE1083EC6E1526997A69", // UDC_CONTRACT_CLASS_HASH as string
            )
            .unwrap(),
            contract_address_salt: FieldElement::from_hex_be("0x1").unwrap(),
            constructor_calldata: vec![FieldElement::from_hex_be("0x1").unwrap()],
            is_query: false,
        };

        let deploy_transaction =
            devnet.json_rpc_client.add_deploy_account_transaction(deploy_account_txn.clone()).await;

        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(deploy_transaction.unwrap().transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::DeployAccount(deploy) = result {
            assert_eq!(
                deploy.transaction_hash,
                FieldElement::from_hex_be(DEPLOY_ACCOUNT_TRANSACTION_HASH).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {result:?}");
        }
    }

    #[tokio::test]
    async fn get_invoke_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let invoke_txn_v1 = BroadcastedInvokeTransactionV1 {
            max_fee: FieldElement::from_hex_be("0xde0b6b3a7640000").unwrap(),
            signature: vec![],
            nonce: FieldElement::from_hex_be("0x0").unwrap(),
            sender_address: FieldElement::from_hex_be("0x0").unwrap(),
            calldata: vec![],
            is_query: false,
        };

        let invoke_transaction = devnet
            .json_rpc_client
            .add_invoke_transaction(starknet_rs_core::types::BroadcastedInvokeTransaction::V1(
                invoke_txn_v1.clone(),
            ))
            .await
            .unwrap();

        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(invoke_transaction.transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = result
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(INVOKE_V1_TRANSACTION_HASH).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {result:?}");
        }
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(FieldElement::from_hex_be("0x0").unwrap())
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
