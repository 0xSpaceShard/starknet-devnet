#![cfg(test)]
pub mod common;

mod call {
    use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt, FunctionCall, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;

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

        match err {
            ProviderError::StarknetError(StarknetError::ContractError(_)) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
