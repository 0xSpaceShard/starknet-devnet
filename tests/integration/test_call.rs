use starknet_rs_core::types::{BlockId, BlockTag, Felt, FunctionCall, StarknetError};
use starknet_rs_providers::jsonrpc::JsonRpcError;
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{ETH_ERC20_CONTRACT_ADDRESS, PREDEPLOYED_ACCOUNT_ADDRESS};

#[tokio::test]
/// This test doesn't rely on devnet.get_balance because it's not supposed to call ERC20
async fn calling_method_of_undeployed_contract() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);
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

// TODO add cairo0 nonexistent method error

#[tokio::test]
async fn calling_nonexistent_contract_method() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let contract_address = Felt::from_hex_unchecked(PREDEPLOYED_ACCOUNT_ADDRESS);
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
        ProviderError::Other(ref error) => {
            let error = error.as_any().downcast_ref::<JsonRpcError>().unwrap();
            assert_eq!(
                (error.code, error.message.as_str()),
                (21, "Requested entrypoint does not exist in the contract")
            );
        }
        _ => panic!("Invalid error: {err:?}"),
    }
}
