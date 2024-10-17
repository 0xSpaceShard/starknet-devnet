#![cfg(test)]
pub mod common;

mod call {
    use std::sync::Arc;

    use starknet_core::constants::{CAIRO_1_ERC20_CONTRACT_CLASS_HASH, ETH_ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::SingleOwnerAccount;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt, FunctionCall, StarknetError};
    use starknet_rs_core::utils::{cairo_short_string_to_felt, get_selector_from_name};
    use starknet_rs_providers::jsonrpc::JsonRpcError;
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH, PREDEPLOYED_ACCOUNT_ADDRESS,
    };
    use crate::common::utils::{
        assert_json_rpc_errors_equal, declare_deploy_v1, deploy_v1, extract_json_rpc_error,
        get_flattened_sierra_contract_and_casm_hash,
    };

    #[tokio::test]
    /// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
    async fn calling_method_of_undeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let entry_point_selector = get_selector_from_name("balanceOf").unwrap();

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
        let entry_point_selector = get_selector_from_name("nonExistentMethod").unwrap();

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

        let selector_hex = entry_point_selector.to_hex_string();
        assert_json_rpc_errors_equal(
            extract_json_rpc_error(err).unwrap(),
            JsonRpcError {
                code: 40,
                message: "Contract error".into(),
                data: Some(serde_json::json!({
                    "contract_address": ETH_ERC20_CONTRACT_ADDRESS.to_hex_string(),
                    "class_hash": CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
                    "selector": selector_hex,
                    "error": format!("Entry point EntryPointSelector({selector_hex}) not found in contract.\n")
                })),
            },
        );
    }

    #[tokio::test]
    async fn call_panicking_method() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            devnet.json_rpc_client.chain_id().await.unwrap(),
            starknet_rs_accounts::ExecutionEncoding::New,
        ));

        let (contract_class, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let (class_hash, contract_address) =
            declare_deploy_v1(account.clone(), contract_class, casm_hash, &[]).await.unwrap();
        let other_contract_address = deploy_v1(account, class_hash, &[]).await.unwrap();

        let top_selector = get_selector_from_name("create_panic_in_another_contract").unwrap();
        let panic_message = cairo_short_string_to_felt("funny_text").unwrap();
        let err = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: top_selector,
                    calldata: vec![other_contract_address, panic_message],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap_err();

        assert_json_rpc_errors_equal(
            extract_json_rpc_error(err).unwrap(),
            JsonRpcError {
                code: 40,
                message: "Contract error".into(),
                data: Some(serde_json::json!({
                    "contract_address": contract_address,
                    "class_hash": class_hash,
                    "selector": top_selector,
                    "error": {
                        "contract_address": other_contract_address,
                        "class_hash": class_hash,
                        "selector": get_selector_from_name("create_panic").unwrap(),
                        "error": "Execution failed. Failure reason: 0x66756e6e795f74657874 ('funny_text').\n"
                    }
                })),
            },
        );
    }
}
