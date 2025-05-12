use std::path::Path;
use std::sync::Arc;

use starknet_rs_accounts::{
    Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_core::types::{BlockId, BlockTag, Felt, StarknetError};
use starknet_rs_core::utils::get_storage_var_address;
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_0_ACCOUNT_CONTRACT_HASH, CHAIN_ID, STRK_ERC20_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    FeeUnit, assert_tx_successful, get_deployable_account_signer, get_simple_contract_artifacts,
    remove_file, send_ctrl_c_signal_and_wait,
};

#[tokio::test]
async fn assert_restartable() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    devnet.restart().await;
}

#[tokio::test]
async fn assert_tx_and_block_not_present_after_restart() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    // generate dummy tx
    let mint_hash = devnet.mint(Felt::ONE, 100).await;
    assert!(devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.is_ok());

    devnet.restart().await;

    match devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await {
        Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => (),
        other => panic!("Unexpected result: {other:?}"),
    }

    match devnet.json_rpc_client.get_block_with_txs(BlockId::Number(1)).await {
        Err(ProviderError::StarknetError(StarknetError::BlockNotFound)) => (),
        other => panic!("Unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn assert_storage_restarted() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    // change storage
    let dummy_address = Felt::ONE;
    let mint_amount = 100;
    devnet.mint(dummy_address, mint_amount).await;

    // define storage retriever
    let storage_key = get_storage_var_address("ERC20_balances", &[dummy_address]).unwrap();
    let get_storage = || {
        devnet.json_rpc_client.get_storage_at(
            STRK_ERC20_CONTRACT_ADDRESS,
            storage_key,
            BlockId::Tag(BlockTag::Latest),
        )
    };

    let storage_value_before = get_storage().await.unwrap();
    assert_eq!(storage_value_before, Felt::from(mint_amount));

    devnet.restart().await;

    let storage_value_after = get_storage().await.unwrap();
    assert_eq!(storage_value_after, Felt::ZERO);
}

#[tokio::test]
async fn assert_account_deployment_reverted() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    // deploy new account
    let account_signer = get_deployable_account_signer();
    let account_factory = OpenZeppelinAccountFactory::new(
        Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
        CHAIN_ID,
        account_signer.clone(),
        devnet.clone_provider(),
    )
    .await
    .unwrap();
    let salt = Felt::ONE;
    let deployment = account_factory.deploy_v3(salt).l1_gas(0).l1_data_gas(1000).l2_gas(1e7 as u64);
    let deployment_address = deployment.address();
    devnet.mint(deployment_address, 1e21 as u128).await;
    let deployment_tx = deployment.send().await.unwrap();

    // assert deployment successful and class associated with deployment address is present
    assert_tx_successful(&deployment_tx.transaction_hash, &devnet.json_rpc_client).await;
    devnet
        .json_rpc_client
        .get_class_at(BlockId::Tag(BlockTag::Latest), deployment_address)
        .await
        .unwrap();

    devnet.restart().await;

    // expect ContractNotFound error since account not present anymore
    match devnet
        .json_rpc_client
        .get_class_at(BlockId::Tag(BlockTag::Latest), deployment_address)
        .await
    {
        Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => (),
        other => panic!("Invalid response: {other:?}"),
    }
}

#[tokio::test]
async fn assert_gas_price_unaffected_by_restart() {
    let expected_l1_gas_price = 1_000_000_u64;
    let expected_l2_gas_price = 2_000_000_u64;
    let expected_data_gas_price = 3_000_000_u64;
    // assert difference to ensure values don't get mixed up in the logic
    assert_ne!(expected_l1_gas_price, expected_data_gas_price);

    let devnet_args = [
        "--gas-price-fri",
        &expected_l1_gas_price.to_string(),
        "--l2-gas-price-fri",
        &expected_l2_gas_price.to_string(),
        "--data-gas-price-fri",
        &expected_data_gas_price.to_string(),
    ];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    // get a predeployed account
    let (signer, address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    ));

    // get class
    let (contract_artifact, casm_hash) = get_simple_contract_artifacts();
    let contract_artifact = Arc::new(contract_artifact);

    // check gas price via fee estimation
    let estimate_before = predeployed_account
        .declare_v3(contract_artifact.clone(), casm_hash)
        .estimate_fee()
        .await
        .unwrap();
    assert_eq!(estimate_before.l1_gas_price, Felt::from(expected_l1_gas_price));
    assert_eq!(estimate_before.l2_gas_price, Felt::from(expected_l2_gas_price));
    assert_eq!(estimate_before.l1_data_gas_price, Felt::from(expected_data_gas_price));

    devnet.restart().await;

    let estimate_after =
        predeployed_account.declare_v3(contract_artifact, casm_hash).estimate_fee().await.unwrap();

    // assert gas_price and fee are equal to the values before restart
    assert_eq!(estimate_after.l1_gas_price, Felt::from(expected_l1_gas_price));
    assert_eq!(estimate_after.l2_gas_price, Felt::from(expected_l2_gas_price));
    assert_eq!(estimate_after.l1_data_gas_price, Felt::from(expected_data_gas_price));
    assert_eq!(estimate_before.overall_fee, estimate_after.overall_fee);
}

#[tokio::test]
async fn assert_predeployed_account_is_prefunded_after_restart() {
    let initial_balance = 1_000_000_u32;
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--initial-balance",
        &initial_balance.to_string(),
    ])
    .await
    .unwrap();

    let predeployed_account_address = devnet.get_first_predeployed_account().await.1;

    let balance_before =
        devnet.get_balance_latest(&predeployed_account_address, FeeUnit::Wei).await.unwrap();
    assert_eq!(balance_before, Felt::from(initial_balance));

    devnet.restart().await;

    let balance_after =
        devnet.get_balance_latest(&predeployed_account_address, FeeUnit::Wei).await.unwrap();
    assert_eq!(balance_before, balance_after);
}

#[tokio::test]
async fn assert_dumping_not_affected_by_restart() {
    let dump_file_name = "dump_after_restart";
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        dump_file_name,
        "--dump-on",
        "exit",
    ])
    .await
    .unwrap();

    devnet.restart().await;

    // send a dummy tx; otherwise there's no dump
    devnet.mint(Felt::ONE, 1).await;

    // assert dump file not already here
    assert!(!Path::new(dump_file_name).exists());

    // assert killing the process can still dump devnet
    send_ctrl_c_signal_and_wait(&devnet.process).await;
    assert!(Path::new(dump_file_name).exists());

    remove_file(dump_file_name);
}

#[tokio::test]
async fn assert_load_not_affecting_restart() {
    let dump_file_name = "dump_before_restart";
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--dump-path",
        dump_file_name,
        "--dump-on",
        "exit",
    ])
    .await
    .unwrap();

    // send a dummy tx; otherwise there's no dump
    let tx_hash = devnet.mint(Felt::ONE, 1).await;

    send_ctrl_c_signal_and_wait(&devnet.process).await;
    assert!(Path::new(dump_file_name).exists());

    let loaded_devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--dump-path", dump_file_name])
            .await
            .unwrap();

    loaded_devnet.restart().await;

    // asserting that restarting really clears the state, without re-executing txs from dump
    match loaded_devnet.json_rpc_client.get_transaction_by_hash(tx_hash).await {
        Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => (),
        other => panic!("Unexpected result: {other:?}"),
    }

    remove_file(dump_file_name);
}

#[tokio::test]
async fn restarting_via_non_rpc() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let dummy_address = Felt::ONE;
    let mint_amount = 100;
    devnet.mint(dummy_address, mint_amount).await;

    devnet.reqwest_client().post_no_body("/restart").await.unwrap();

    let balance_after = devnet.get_balance_latest(&dummy_address, FeeUnit::Wei).await.unwrap();
    assert_eq!(balance_after, Felt::ZERO);
}
