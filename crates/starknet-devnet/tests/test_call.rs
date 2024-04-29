pub mod common;

mod call {
    use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;

    #[tokio::test]
    /// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
    async fn calling_method_of_undeployed_contract() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"]).await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap();

        let undeployed_address = FieldElement::from_hex_be("0x1234").unwrap();
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: undeployed_address,
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }

        // Create new block
        let block_hash = devnet.create_block().await;

        // Test with newly created block's hash
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: undeployed_address,
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Hash(block_hash.unwrap()),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }

        // Test with block number
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: undeployed_address,
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Number(1),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn calling_nonexistent_contract_method() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"]).await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("nonExistentMethod").unwrap();

        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS)
                        .unwrap(),
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractError(_)) => (),
            _ => panic!("Invalid error: {err:?}"),
        }

        let block_hash=  devnet.create_block().await;
        // Test with block hash
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS)
                        .unwrap(),
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Hash(block_hash.unwrap()),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractError(_)) => (),
            _ => panic!("Invalid error: {err:?}"),
        }

        // Test with block number
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS)
                        .unwrap(),
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Number(1),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractError(_)) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}