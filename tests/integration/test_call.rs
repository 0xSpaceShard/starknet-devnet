use starknet_rs_accounts::SingleOwnerAccount;
use starknet_rs_core::types::{
    BlockId, BlockTag, ContractErrorData, Felt, FunctionCall, StarknetError,
};
use starknet_rs_core::utils::{cairo_short_string_to_felt, get_selector_from_name};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH, ETH_ERC20_CONTRACT_ADDRESS, PREDEPLOYED_ACCOUNT_ADDRESS,
};
use crate::common::utils::{
    assert_contains, declare_v3_deploy_v3, deploy_v3, extract_message_error, extract_nested_error,
    get_flattened_sierra_contract_and_casm_hash,
};

#[tokio::test]
/// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
async fn calling_method_of_undeployed_contract() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);
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
async fn calling_nonexistent_cairo0_contract_method() {
    let devnet_args = ["--account-class", "cairo0"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);
    let entry_point_selector =
        starknet_rs_core::utils::get_selector_from_name("nonExistentMethod").unwrap();

    let err = devnet
        .json_rpc_client
        .call(
            FunctionCall {
                contract_address,
                entry_point_selector,
                calldata: vec![contract_address],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect_err("Should have failed");

    match err {
        ProviderError::StarknetError(StarknetError::EntrypointNotFound) => {}
        _ => {
            panic!("Invalid error received {:?}", err);
        }
    }
}

#[tokio::test]
async fn calling_nonexistent_cairo1_contract_method() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);
    let entry_point_selector = get_selector_from_name("nonExistentMethod").unwrap();

    let called_contract_address = ETH_ERC20_CONTRACT_ADDRESS;
    let err = devnet
        .json_rpc_client
        .call(
            FunctionCall {
                contract_address: called_contract_address,
                entry_point_selector,
                calldata: vec![contract_address],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect_err("Should have failed");

    match err {
        ProviderError::StarknetError(StarknetError::EntrypointNotFound) => {}
        _ => {
            panic!("Invalid error received {:?}", err);
        }
    }
}

#[tokio::test]
async fn call_panicking_method() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        starknet_rs_accounts::ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (class_hash, contract_address) =
        declare_v3_deploy_v3(&account, contract_class, casm_hash, &[]).await.unwrap();

    let other_contract_address = deploy_v3(&account, class_hash, &[]).await.unwrap();

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

    match err {
        ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
            revert_error,
        })) => {
            let root = extract_nested_error(&revert_error);
            assert_eq!(root.contract_address, contract_address);
            assert_eq!(root.class_hash, class_hash);
            assert_eq!(root.selector, top_selector);

            let inner = extract_nested_error(&root.error);
            assert_eq!(inner.contract_address, other_contract_address);
            assert_eq!(inner.selector, get_selector_from_name("create_panic").unwrap());
            assert_eq!(inner.class_hash, class_hash);

            let error_msg = extract_message_error(&inner.error);
            assert_contains(error_msg, &panic_message.to_hex_string());
        }
        _ => panic!("Invalid error received {err:?}"),
    }
}
