use std::sync::Arc;

use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{BlockId, BlockTag, EthAddress, Felt, MsgFromL1, StarknetError};
use starknet_rs_core::utils::{UdcUniqueness, get_udc_deployed_address};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{CHAIN_ID, L1_HANDLER_SELECTOR, MESSAGING_WHITELISTED_L1_CONTRACT};
use crate::common::utils::get_messaging_contract_artifacts;

#[tokio::test]
async fn estimate_message_fee() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    // get class
    let (contract_artifact, casm_hash) = get_messaging_contract_artifacts();
    let contract_artifact = Arc::new(contract_artifact);
    let class_hash = contract_artifact.class_hash();

    // declare class
    account.declare_v3(contract_artifact, casm_hash).nonce(Felt::ZERO).send().await.unwrap();

    // deploy instance of class
    let contract_factory = ContractFactory::new(class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![];
    let contract_address = get_udc_deployed_address(
        salt,
        class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory
        .deploy_v3(constructor_calldata, salt, false)
        .nonce(Felt::ONE)
        .send()
        .await
        .expect("Cannot deploy");

    let res = devnet
        .json_rpc_client
        .estimate_message_fee(
            MsgFromL1 {
                from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                to_address: contract_address,
                entry_point_selector: Felt::from_hex_unchecked(L1_HANDLER_SELECTOR),
                payload: [(1_u32).into(), (10_u32).into()].to_vec(),
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .unwrap();

    assert_eq!(res.l1_gas_consumed, 16030);
    assert_eq!(res.l2_gas_consumed, 0);
}

#[tokio::test]
async fn estimate_message_fee_contract_not_found() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let err = devnet
        .json_rpc_client
        .estimate_message_fee(
            MsgFromL1 {
                from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                to_address: Felt::ONE,
                entry_point_selector: Felt::from_hex_unchecked(L1_HANDLER_SELECTOR),
                payload: [(1_u32).into(), (10_u32).into()].to_vec(),
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect_err("Error expected");

    match err {
        ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
        _ => panic!("Invalid error: {err:?}"),
    }
}

#[tokio::test]
async fn estimate_message_fee_block_not_found() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");

    let err = devnet
        .json_rpc_client
        .estimate_message_fee(
            MsgFromL1 {
                from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                to_address: Felt::ONE,
                entry_point_selector: Felt::from_hex_unchecked(L1_HANDLER_SELECTOR),
                payload: [(1_u32).into(), (10_u32).into()].to_vec(),
            },
            BlockId::Number(101),
        )
        .await
        .expect_err("Error expected");

    match err {
        ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
        _ => panic!("Invalid error: {err:?}"),
    }
}
