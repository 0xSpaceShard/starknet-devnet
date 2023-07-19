pub mod common;

mod call {
    use starknet_core::constants::{DEVNET_DEFAULT_INITIAL_BALANCE, ERC20_CONTRACT_ADDRESS};
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::BackgroundDevnet;

    #[tokio::test]
    async fn calling_method_of_undeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
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
    }

    #[tokio::test]
    async fn calling_nonexistent_contract_method() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("nonExistentMethod").unwrap();

        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap(),
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractError) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn getting_balance_of_predeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap();

        let retrieved_result = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap(),
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("Failed to call contract");

        let expected_hex_balance = format!("0x{DEVNET_DEFAULT_INITIAL_BALANCE:x}");
        let expected_balance = FieldElement::from_hex_be(expected_hex_balance.as_str()).unwrap();
        let expected_result = vec![expected_balance, FieldElement::ZERO]; // uint256
        assert_eq!(retrieved_result, expected_result);
    }
}
