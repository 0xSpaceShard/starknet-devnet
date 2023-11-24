pub mod common;

mod call {
    use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall, StarknetError};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
    };

    #[tokio::test]
    /// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
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
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                ..
            }) => (),
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
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::ContractError),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn getting_balance_of_predeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_result = devnet.get_balance(&contract_address).await.unwrap();

        let expected_hex_balance = format!("0x{PREDEPLOYED_ACCOUNT_INITIAL_BALANCE:x}");
        let expected_balance = FieldElement::from_hex_be(expected_hex_balance.as_str()).unwrap();
        assert_eq!(retrieved_result, expected_balance);
    }
}
