use std::sync::Arc;

use serde_json::json;
use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{BlockId, BlockTag, Call, Felt, FunctionCall};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::Provider;
use starknet_rs_signers::LocalWallet;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    ARGENT_ACCOUNT_CLASS_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
    CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH, INTEGRATION_SEPOLIA_HTTP_URL,
};
use crate::common::utils::{
    FeeUnit, assert_contains, assert_tx_succeeded_accepted, deploy_argent_account,
    deploy_oz_account, get_simple_contract_artifacts,
};

pub async fn get_predeployed_accounts(
    devnet: &BackgroundDevnet,
    params: serde_json::Value,
) -> serde_json::Value {
    devnet.send_custom_rpc("devnet_getPredeployedAccounts", params).await.unwrap()
}

#[tokio::test]
async fn spawnable_with_cairo0() {
    BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo0"]).await.unwrap();
}

#[tokio::test]
async fn spawnable_with_cairo1() {
    BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"]).await.unwrap();
}

#[tokio::test]
async fn spawnable_with_custom_account_cairo_1() {
    let cli_args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
    BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();
}

/// Common body for tests defined below
async fn correct_artifact_test_body(
    devnet_args: &[&str],
    expected_hash_hex: &str,
) -> Result<(), anyhow::Error> {
    let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await?;

    let (_, account_address) = devnet.get_first_predeployed_account().await;
    let retrieved_class_hash = devnet
        .json_rpc_client
        .get_class_hash_at(BlockId::Tag(BlockTag::Latest), account_address)
        .await?;
    let expected_hash = Felt::from_hex_unchecked(expected_hash_hex);
    anyhow::ensure!(retrieved_class_hash == expected_hash);

    let config = devnet.get_config().await;
    let config_class_hash_hex = config["account_contract_class_hash"]
        .as_str()
        .ok_or(anyhow::anyhow!("contract class hash not found"))?;
    anyhow::ensure!(Felt::from_hex_unchecked(config_class_hash_hex) == expected_hash);
    Ok(())
}

#[tokio::test]
async fn correct_cairo1_artifact() {
    let cli_args = ["--account-class", "cairo1"];
    correct_artifact_test_body(&cli_args, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).await.unwrap();
}

#[tokio::test]
async fn correct_custom_artifact() {
    let cli_args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
    correct_artifact_test_body(&cli_args, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).await.unwrap();
}

#[tokio::test]
async fn can_deploy_new_cairo1_oz_account() {
    let cli_args = ["--account-class", "cairo1"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let (account_deployment, signer) = deploy_oz_account(&devnet).await.unwrap();
    assert_tx_succeeded_accepted(&account_deployment.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_address = account_deployment.contract_address;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

#[tokio::test]
async fn can_deploy_new_cairo1_oz_account_when_cairo0_selected() {
    let cli_args = ["--account-class", "cairo0"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let (account_deployment, signer) = deploy_oz_account(&devnet).await.unwrap();
    assert_tx_succeeded_accepted(&account_deployment.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_address = account_deployment.contract_address;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

#[tokio::test]
async fn can_deploy_new_custom_oz_account() {
    let cli_args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let (account_deployment, signer) = deploy_oz_account(&devnet).await.unwrap();
    assert_tx_succeeded_accepted(&account_deployment.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_address = account_deployment.contract_address;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

#[tokio::test]
async fn argent_account_undeployable_by_default() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let account_hash = Felt::from_hex_unchecked(ARGENT_ACCOUNT_CLASS_HASH);
    let error = deploy_argent_account(&devnet, account_hash).await.unwrap_err();
    assert_contains(
        &error.to_string(),
        &format!("Class with hash {ARGENT_ACCOUNT_CLASS_HASH} is not declared"),
    )
    .unwrap();
}

#[tokio::test]
/// Relying on forking: the origin network is expected to have the account class declared.
async fn can_deploy_instance_of_argent_account_via_fork() {
    let cli_args = ["--fork-network", INTEGRATION_SEPOLIA_HTTP_URL];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let account_hash = Felt::from_hex_unchecked(ARGENT_ACCOUNT_CLASS_HASH);
    let (account_deployment, signer) = deploy_argent_account(&devnet, account_hash).await.unwrap();
    assert_tx_succeeded_accepted(&account_deployment.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_address = account_deployment.contract_address;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

#[tokio::test]
async fn can_deploy_new_argent_account_from_predeclared_class() {
    let devnet_args = ["--predeclare-argent"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let account_hash = Felt::from_hex_unchecked(ARGENT_ACCOUNT_CLASS_HASH);
    let (account_deployment, signer) = deploy_argent_account(&devnet, account_hash).await.unwrap();
    assert_tx_succeeded_accepted(&account_deployment.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();

    let account_address = account_deployment.contract_address;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

async fn can_declare_deploy_invoke_cairo1_using_account(
    devnet: &BackgroundDevnet,
    signer: &LocalWallet,
    account_address: Felt,
) {
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::New,
    ));

    let (contract_class, casm_hash) = get_simple_contract_artifacts();

    // declare the contract
    let declaration_result = account
        .declare_v3(Arc::new(contract_class), casm_hash)
        .gas_estimate_multiplier(1.0)
        .gas_price_estimate_multiplier(1.0)
        .send()
        .await
        .unwrap();

    // deploy the contract
    let contract_factory = ContractFactory::new(declaration_result.class_hash, account.clone());
    let initial_value = Felt::from(10_u32);
    let ctor_args = vec![initial_value];
    contract_factory.deploy_v3(ctor_args.clone(), Felt::ZERO, false).send().await.unwrap();

    // generate the address of the newly deployed contract
    let contract_address = get_udc_deployed_address(
        Felt::ZERO,
        declaration_result.class_hash,
        &starknet_rs_core::utils::UdcUniqueness::NotUnique,
        &ctor_args,
    );

    // invoke on forked devnet
    let increment = Felt::from(5_u32);
    let contract_invoke = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![increment, Felt::ZERO],
    }];

    let invoke_result = account.execute_v3(contract_invoke.clone()).send().await.unwrap();

    assert_tx_succeeded_accepted(&invoke_result.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn can_declare_deploy_invoke_using_predeployed_cairo1() {
    let cli_args = ["--account-class", "cairo1"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

#[tokio::test]
async fn can_declare_deploy_invoke_using_predeployed_custom() {
    let cli_args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    can_declare_deploy_invoke_cairo1_using_account(&devnet, &signer, account_address).await;
}

async fn assert_supports_isrc6(devnet: &BackgroundDevnet, account_address: Felt) {
    // https://github.com/OpenZeppelin/cairo-contracts/blob/89a450a88628ec3b86273f261b2d8d1ca9b1522b/src/account/interface.cairo#L7
    let interface_id_hex = "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd";
    let interface_id = Felt::from_hex_unchecked(interface_id_hex);

    let call = FunctionCall {
        contract_address: account_address,
        entry_point_selector: get_selector_from_name("supports_interface").unwrap(),
        calldata: vec![interface_id],
    };

    let supports = devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
    assert_eq!(supports, vec![Felt::ONE]);
}

#[tokio::test]
async fn test_interface_support_of_predeployed_account() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();
    let (_, account_address) = devnet.get_first_predeployed_account().await;

    assert_supports_isrc6(&devnet, account_address).await;
}

#[tokio::test]
async fn test_interface_support_of_newly_deployed_account() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (account_deployment, _) = deploy_oz_account(&devnet).await.unwrap();

    assert_supports_isrc6(&devnet, account_deployment.contract_address).await;
}

#[tokio::test]
async fn test_get_predeployed_accounts_balances() {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&[
        "--accounts",
        "10",
        "--initial-balance",
        "1",
    ])
    .await
    .unwrap();

    let accounts = get_predeployed_accounts(&devnet, json!({})).await;
    for account in accounts.as_array().unwrap() {
        assert!(account["balance"].is_null());
    }

    let accounts_without_balance =
        get_predeployed_accounts(&devnet, json!({"with_balance": false})).await;
    for account in accounts_without_balance.as_array().unwrap() {
        assert!(account["balance"].is_null());
    }

    let accounts_with_balance =
        get_predeployed_accounts(&devnet, json!({"with_balance": true})).await;
    assert_eq!(accounts_with_balance.as_array().unwrap().len(), 10);
    for account in accounts_with_balance.as_array().unwrap() {
        assert_eq!(
            account["balance"],
            json!(
                {
                    "eth": { "amount":  "1", "unit": "WEI" },
                    "strk": { "amount":  "1", "unit": "FRI" },
                }
            )
        );
    }

    // increase balances and check again
    for account in accounts_with_balance.as_array().unwrap() {
        let address = &Felt::from_hex_unchecked(account["address"].as_str().unwrap());
        devnet.mint_unit(address, 1, FeeUnit::Wei).await;
        devnet.mint_unit(address, 1, FeeUnit::Fri).await;
    }

    let accounts_with_balance =
        get_predeployed_accounts(&devnet, json!({"with_balance": true})).await;
    assert_eq!(accounts_with_balance.as_array().unwrap().len(), 10);
    for account in accounts_with_balance.as_array().unwrap() {
        assert_eq!(
            account["balance"],
            json!(
                {
                    "eth": { "amount":  "2", "unit": "WEI" },
                    "strk": { "amount":  "2", "unit": "FRI" },
                }
            )
        );
    }
}
