#![cfg(test)]
pub mod common;

mod get_class_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::CasmContractClass;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, ContractClass, Felt, StarknetError};
    use starknet_rs_providers::jsonrpc::JsonRpcError;
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::utils::{
        assert_cairo1_classes_equal, get_events_contract_in_sierra_and_compiled_class_hash,
    };

    #[tokio::test]
    async fn test_getting_class_at() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_getting_class() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

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

        let json_string =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/simple_contract.json")
                .unwrap();
        let contract_class: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_str(&json_string).unwrap());

        // declare the contract
        let declaration_result = predeployed_account
            .declare_legacy(contract_class.clone())
            .max_fee(Felt::from(1e18 as u128))
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
            .declare_v2(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(Felt::from(1e18 as u128))
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
        let contract_address = Felt::from_hex_unchecked("0x22");

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
            .declare_v2(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // getting class at the following block IDs should NOT be successful
        let declaration_block_number = BlockId::Number(original_block.block_number + 1);
        for (block_id, expected_err) in [
            (BlockId::Number(original_block.block_number), StarknetError::ClassHashNotFound),
            (BlockId::Hash(original_block.block_hash), StarknetError::ClassHashNotFound),
            (BlockId::Tag(BlockTag::Latest), StarknetError::ClassHashNotFound),
            (declaration_block_number, StarknetError::BlockNotFound),
        ] {
            let retrieved =
                devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
            match retrieved {
                Err(ProviderError::StarknetError(err)) => assert_eq!(err, expected_err),
                other => panic!("Unexpected response at block_id={block_id:?}: {other:?}"),
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

    #[tokio::test]
    async fn test_getting_class_after_block_abortion() {
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
            .declare_v2(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let abortable_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        devnet
            .send_custom_rpc(
                "devnet_abortBlocks",
                serde_json::json!({ "starting_block_id": BlockId::Hash(abortable_block.block_hash) }),
            )
            .await
            .unwrap();

        // Getting class at the following block IDs should NOT be successful after abortion; these
        // blocks exist, but their states don't contain the class.
        for (block_id, expected_err) in [
            // this block's state is invalidated
            (BlockId::Number(abortable_block.block_number), StarknetError::BlockNotFound),
            (BlockId::Hash(abortable_block.block_hash), StarknetError::BlockNotFound),
            (BlockId::Tag(BlockTag::Latest), StarknetError::ClassHashNotFound),
            (BlockId::Tag(BlockTag::Pending), StarknetError::ClassHashNotFound),
        ] {
            let retrieved =
                devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
            match retrieved {
                Err(ProviderError::StarknetError(err)) => assert_eq!(err, expected_err),
                other => panic!("Unexpected response at block_id={block_id:?}: {other:?}"),
            }
        }

        let latest_block_hash = devnet.create_block().await.unwrap();

        // getting class at the following block IDs should NOT be successful after creating a block
        // that has the same number that the aborted block had
        for (block_id, expected_err) in [
            (BlockId::Number(abortable_block.block_number), StarknetError::ClassHashNotFound),
            (BlockId::Hash(abortable_block.block_hash), StarknetError::BlockNotFound),
            (BlockId::Hash(latest_block_hash), StarknetError::ClassHashNotFound),
            (BlockId::Tag(BlockTag::Latest), StarknetError::ClassHashNotFound),
            (BlockId::Tag(BlockTag::Pending), StarknetError::ClassHashNotFound),
        ] {
            let retrieved =
                devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
            match retrieved {
                Err(ProviderError::StarknetError(err)) => assert_eq!(err, expected_err),
                other => panic!("Unexpected response at block_id={block_id:?}: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn test_getting_compiled_casm_for_cairo0_or_non_existing_hash_have_to_return_class_hash_not_found_error()
     {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo0"])
            .await
            .expect("Could not start Devnet");

        let (_, account_address) = devnet.get_first_predeployed_account().await;

        let block_id = BlockId::Tag(BlockTag::Latest);

        let class_hash =
            devnet.json_rpc_client.get_class_hash_at(block_id, account_address).await.unwrap();

        // Felt::ONE is non existing class hash
        for el in [class_hash, Felt::ONE] {
            match get_compiled_casm(&devnet, block_id, el).await.unwrap_err() {
                StarknetError::ClassHashNotFound => {}
                other => panic!("Unexpected error {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_getting_compiled_casm_for_cairo_1_have_to_succeed() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
            .await
            .expect("Could not start Devnet");

        let (_, account_address) = devnet.get_first_predeployed_account().await;

        let block_id = BlockId::Tag(BlockTag::Latest);

        let class_hash =
            devnet.json_rpc_client.get_class_hash_at(block_id, account_address).await.unwrap();

        get_compiled_casm(&devnet, block_id, class_hash).await.unwrap();
    }

    async fn get_compiled_casm(
        devnet: &BackgroundDevnet,
        block_id: BlockId,
        class_hash: Felt,
    ) -> Result<CasmContractClass, StarknetError> {
        devnet
            .send_custom_rpc(
                "starknet_getCompiledCasm",
                json!({
                    "block_id": block_id,
                    "class_hash": format!("{:#x}", class_hash),
                }),
            )
            .await
            .map(|json_value| serde_json::from_value::<CasmContractClass>(json_value).unwrap())
            .map_err(|err| {
                let json_rpc_error = JsonRpcError {
                    code: err.code.code(),
                    message: err.message.to_string(),
                    data: err.data,
                };

                (&json_rpc_error).try_into().unwrap()
            })
    }
}
