pub mod common;

mod get_class_tests {
    use std::sync::Arc;

    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, ContractClass, FieldElement, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::utils::{
        assert_cairo1_classes_equal, get_events_contract_in_sierra_and_compiled_class_hash,
        resolve_path,
    };

    #[tokio::test]
    async fn test_getting_class_at() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_getting_class() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();

        devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), retrieved_hash)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_getting_class_of_declared_cairo0_contract() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        ));

        let json_string = std::fs::read_to_string(resolve_path(
            "../starknet-devnet-core/test_artifacts/cairo_0_test.json",
        ))
        .unwrap();
        let contract_class: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_str(&json_string).unwrap());

        // declare the contract
        let declaration_result = predeployed_account
            .declare_legacy(contract_class.clone())
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let retrieved_class = devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), declaration_result.class_hash)
            .await
            .unwrap();

        assert_eq!(retrieved_class, ContractClass::Legacy(contract_class.compress().unwrap()));
    }

    #[tokio::test]
    async fn test_getting_class_of_declared_cairo1_contract() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let retrieved_class = devnet
            .json_rpc_client
            .get_class(BlockId::Tag(BlockTag::Latest), declaration_result.class_hash)
            .await
            .unwrap();

        assert_cairo1_classes_equal(&retrieved_class, &ContractClass::Sierra(contract_class))
            .unwrap();
    }

    #[tokio::test]
    async fn test_getting_class_at_invalid_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be("0x22").unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn test_getting_class_at_various_blocks() {
        let devnet_args = ["--state-archive-capacity", "full"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let declaration_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        // create an extra block so the declaration block is no longer the latest
        devnet.create_block().await.unwrap();

        // getting class at the following block IDs should be successful
        let expected_class = ContractClass::Sierra(contract_class);
        for block_id in [
            BlockId::Tag(BlockTag::Latest),
            BlockId::Tag(BlockTag::Pending),
            BlockId::Number(declaration_block.block_number),
            BlockId::Number(declaration_block.block_number + 1),
            BlockId::Hash(declaration_block.block_hash),
        ] {
            let retrieved_class = devnet
                .json_rpc_client
                .get_class(block_id, declaration_result.class_hash)
                .await
                .unwrap();

            assert_cairo1_classes_equal(&retrieved_class, &expected_class).unwrap();
        }

        // getting class at the following block IDs should NOT be successful
        for block_id in [BlockId::Number(declaration_block.block_number - 1)] {
            let retrieved =
                devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
            match retrieved {
                Err(ProviderError::StarknetError(StarknetError::ClassHashNotFound)) => (),
                other => panic!("Unexpected response: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn test_getting_class_with_blocks_on_demand() {
        let devnet_args = ["--state-archive-capacity", "full", "--block-generation-on", "demand"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        let original_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // getting class at the following block IDs should NOT be successful
        for block_id in [
            BlockId::Number(original_block.block_number),
            BlockId::Hash(original_block.block_hash),
            BlockId::Tag(BlockTag::Latest),
        ] {
            let retrieved =
                devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
            match retrieved {
                Err(ProviderError::StarknetError(StarknetError::ClassHashNotFound)) => (),
                other => panic!("Unexpected response at block_id={block_id:?}: {other:?}"),
            }
        }

        // getting class from the future block number should yield a different error
        let declaration_block_number = BlockId::Number(original_block.block_number + 1);
        {
            let retrieved = devnet
                .json_rpc_client
                .get_class(declaration_block_number, declaration_result.class_hash)
                .await;
            match retrieved {
                Err(ProviderError::StarknetError(StarknetError::BlockNotFound)) => (),
                other => panic!("Unexpected response: {other:?}"),
            }
        }

        // getting class at the following block IDs should be successful even before block creation
        let expected_class = ContractClass::Sierra(contract_class);
        for block_id in [BlockId::Tag(BlockTag::Pending)] {
            let retrieved_class = devnet
                .json_rpc_client
                .get_class(block_id, declaration_result.class_hash)
                .await
                .unwrap();

            assert_cairo1_classes_equal(&retrieved_class, &expected_class).unwrap();
        }

        let declaration_block_hash = devnet.create_block().await.unwrap();

        // getting class at the following block IDs should be successful after block creation
        for block_id in [
            BlockId::Tag(BlockTag::Latest),
            BlockId::Tag(BlockTag::Pending),
            declaration_block_number,
            BlockId::Hash(declaration_block_hash),
        ] {
            let retrieved_class = devnet
                .json_rpc_client
                .get_class(block_id, declaration_result.class_hash)
                .await
                .unwrap();

            assert_cairo1_classes_equal(&retrieved_class, &expected_class).unwrap();
        }
    }
}
