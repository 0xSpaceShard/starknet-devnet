#![cfg(test)]
pub mod common;

mod call {
    use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt, FunctionCall, StarknetError};
    use starknet_rs_providers::jsonrpc::JsonRpcError;
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::utils::{assert_json_rpc_errors_equal, extract_json_rpc_error};

    #[tokio::test]
    /// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
    async fn calling_method_of_undeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap();

        let undeployed_address = Felt::from_hex_unchecked("0x1234");
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
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("nonExistentMethod").unwrap();

        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address: ETH_ERC20_CONTRACT_ADDRESS,
                    entry_point_selector,
                    calldata: vec![contract_address],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Should have failed");

        let json_rpc_error = extract_json_rpc_error(err).unwrap();
        assert_json_rpc_errors_equal(
            json_rpc_error,
            JsonRpcError {
                code: 40,
                message: "Contract error".into(),
                data: Some(serde_json::json!({
                    "contract_address": "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                    "class_hash": "0x46ded64ae2dead6448e247234bab192a9c483644395b66f2155f2614e5804b0",
                    "selector": "0x2a083659c1bce11200ae5e0a51a3da1830c2ed108c2243f77ced344cf95357f",
                    "error": "Entry point EntryPointSelector(0x2a083659c1bce11200ae5e0a51a3da1830c2ed108c2243f77ced344cf95357f) not found in contract.\n"
                })),
            },
        );
    }
}
