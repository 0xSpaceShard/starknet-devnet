use std::sync::Arc;

use serde_json::json;
use starknet_core::constants::DEVNET_DEFAULT_GAS_PRICE;
use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::chain_id::SEPOLIA;
use starknet_rs_core::types::{Felt, ResourcePrice, StarknetError};
use starknet_rs_providers::ProviderError;
use starknet_rs_signers::Signer;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_CONTRACT_PATH, INTEGRATION_SAFE_BLOCK, INTEGRATION_SEPOLIA_HTTP_URL,
};
use crate::common::errors::RpcError;
use crate::common::fees::assert_difference_if_validation;
use crate::common::utils::{
    assert_tx_successful, felt_to_u128, get_flattened_sierra_contract_and_casm_hash,
    get_simple_contract_in_sierra_and_compiled_class_hash, iter_to_hex_felt, to_hex_felt,
    to_num_as_hex,
};

trait SetGasPrice {
    async fn set_gas_price(
        &self,
        gas_price: &serde_json::Value,
        generate_block: bool,
    ) -> Result<serde_json::Value, RpcError>;
}

impl SetGasPrice for BackgroundDevnet {
    async fn set_gas_price(
        &self,
        gas_price: &serde_json::Value,
        generate_block: bool,
    ) -> Result<serde_json::Value, RpcError> {
        let mut req_obj = gas_price.clone();
        if generate_block {
            req_obj["generate_block"] = json!(true);
        }

        self.send_custom_rpc("devnet_setGasPrice", req_obj).await
    }
}

/// Test scenario for gas modification involving simulateTransactions:
/// 1. Execute simulateTransactions with a declare transaction and check gas fees.
/// 2. Set the gas values.
/// 3. Execute simulateTransactions again and check gas fees.
async fn set_gas_scenario(devnet: BackgroundDevnet, expected_chain_id: &str) {
    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        Felt::from_hex_unchecked(expected_chain_id),
        ExecutionEncoding::New,
    );

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

    let max_fee = Felt::ZERO;
    let max_gas = 0;
    // TODO max_gas_price?
    let nonce = Felt::ZERO;

    let declaration = account
        .declare_v3(Arc::new(flattened_contract_artifact.clone()), casm_hash)
        .gas(max_gas)
        .nonce(nonce)
        .prepared()
        .unwrap();
    let signature = signer.sign_hash(&declaration.transaction_hash(false)).await.unwrap();

    let sender_address_hex = to_hex_felt(&account_address);
    let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
        json!({
            "block_id": "latest",
            "simulation_flags": simulation_flags,
            "transactions": [
                {
                    "type": "DECLARE",
                    "sender_address": sender_address_hex,
                    "compiled_class_hash": to_hex_felt(&casm_hash),
                    "max_fee": to_hex_felt(&max_fee),
                    "version": "0x2",
                    "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                    "nonce": to_num_as_hex(&nonce),
                    "contract_class": flattened_contract_artifact,
                }
            ]
        })
    };

    let chain_id = &devnet.send_custom_rpc("starknet_chainId", json!({})).await.unwrap();
    assert_eq!(chain_id, expected_chain_id);

    let params_skip_fee_charge = get_params(&["SKIP_FEE_CHARGE"]);
    let resp_no_flags = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_fee_charge.clone())
        .await
        .unwrap()[0];
    assert_eq!(
        resp_no_flags["fee_estimation"]["gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_GAS_PRICE)
    );
    assert_eq!(
        resp_no_flags["fee_estimation"]["data_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_GAS_PRICE)
    );
    assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0x7398c659d800");

    let params_skip_validation_and_fee_charge = get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]);
    let resp_skip_validation = &devnet
        .send_custom_rpc(
            "starknet_simulateTransactions",
            params_skip_validation_and_fee_charge.clone(),
        )
        .await
        .unwrap()[0];
    assert_eq!(
        resp_skip_validation["fee_estimation"]["gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_GAS_PRICE)
    );
    assert_eq!(
        resp_skip_validation["fee_estimation"]["data_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_GAS_PRICE)
    );
    assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0x736a356c0800");

    assert_difference_if_validation(
        resp_no_flags,
        resp_skip_validation,
        &sender_address_hex,
        max_fee == Felt::ZERO,
    );

    let wei_price = 9e18 as u128;
    let wei_price_data = 8e18 as u128;
    let gas_request = json!({
        "gas_price_wei": wei_price,
        "data_gas_price_wei": wei_price_data,
        "gas_price_fri": 7e18 as u128,
        "data_gas_price_fri": 6e18 as u128,
    });
    let gas_response = &devnet.set_gas_price(&gas_request, true).await.unwrap();

    assert_eq!(gas_response, &gas_request);

    let chain_id = &devnet.send_custom_rpc("starknet_chainId", json!({})).await.unwrap();
    assert_eq!(chain_id, expected_chain_id);

    let resp_no_flags = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_fee_charge)
        .await
        .unwrap()[0];

    assert_eq!(resp_no_flags["fee_estimation"]["gas_price"], to_hex_felt(&wei_price));
    assert_eq!(resp_no_flags["fee_estimation"]["data_gas_price"], to_hex_felt(&wei_price_data));
    assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0x261b37abed7125c0000");

    let resp_skip_validation = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_validation_and_fee_charge)
        .await
        .unwrap()[0];
    assert_eq!(resp_skip_validation["fee_estimation"]["gas_price"], to_hex_felt(&wei_price));
    assert_eq!(
        resp_skip_validation["fee_estimation"]["data_gas_price"],
        to_hex_felt(&wei_price_data)
    );
    assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0x260b9ade6354d540000");

    assert_difference_if_validation(
        resp_no_flags,
        resp_skip_validation,
        &sender_address_hex,
        max_fee == Felt::ZERO,
    );
}

#[tokio::test]
async fn set_gas() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // Testnet gas modification test scenario
    set_gas_scenario(devnet, &SEPOLIA.to_hex_string()).await;
}

#[tokio::test]
async fn set_gas_fork() {
    let fork_block = &INTEGRATION_SAFE_BLOCK.to_string();
    let cli_args = ["--fork-network", INTEGRATION_SEPOLIA_HTTP_URL, "--fork-block", fork_block];
    let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    // Sepolia fork gas modification test scenario
    set_gas_scenario(fork_devnet, "0x534e5f494e544547524154494f4e5f5345504f4c4941").await;
}

#[tokio::test]
async fn set_gas_check_blocks() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let default_gas_price = ResourcePrice {
        price_in_wei: u128::from(DEVNET_DEFAULT_GAS_PRICE).into(),
        price_in_fri: u128::from(DEVNET_DEFAULT_GAS_PRICE).into(),
    };

    // First update - don't generate new block
    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 0);
    assert_eq!(latest_block.l1_gas_price, default_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, default_gas_price);

    let first_update_gas_price =
        ResourcePrice { price_in_wei: (9e18 as u128).into(), price_in_fri: (7e18 as u128).into() };
    let first_update_data_gas_price =
        ResourcePrice { price_in_wei: (8e18 as u128).into(), price_in_fri: (6e18 as u128).into() };
    let gas_request = json!({
        "gas_price_wei": felt_to_u128(first_update_gas_price.price_in_wei),
        "data_gas_price_wei": felt_to_u128(first_update_data_gas_price.price_in_wei),
        "gas_price_fri": felt_to_u128(first_update_gas_price.price_in_fri),
        "data_gas_price_fri": felt_to_u128(first_update_data_gas_price.price_in_fri),
    });
    let gas_response = devnet.set_gas_price(&gas_request, false).await.unwrap();
    assert_eq!(gas_response, gas_request);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 0);

    let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
    assert_eq!(pending_block.l1_gas_price, default_gas_price);
    assert_eq!(pending_block.l1_data_gas_price, default_gas_price);

    devnet.create_block().await.unwrap();

    let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
    assert_eq!(pending_block.l1_gas_price, first_update_gas_price);
    assert_eq!(pending_block.l1_data_gas_price, first_update_data_gas_price);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 1);
    assert_eq!(latest_block.l1_gas_price, first_update_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, first_update_data_gas_price);

    // Second update - generate new block
    let second_update_gas_price =
        ResourcePrice { price_in_wei: (8e18 as u128).into(), price_in_fri: (6e18 as u128).into() };
    let second_update_data_gas_price =
        ResourcePrice { price_in_wei: (7e18 as u128).into(), price_in_fri: (5e18 as u128).into() };
    let gas_price = json!({
        "gas_price_wei": felt_to_u128(second_update_gas_price.price_in_wei),
        "data_gas_price_wei": felt_to_u128(second_update_data_gas_price.price_in_wei),
        "gas_price_fri": felt_to_u128(second_update_gas_price.price_in_fri),
        "data_gas_price_fri": felt_to_u128(second_update_data_gas_price.price_in_fri),
    });
    let gas_response = devnet.set_gas_price(&gas_price, true).await.unwrap();
    assert_eq!(gas_response, gas_price);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 2);
    assert_eq!(latest_block.l1_gas_price, second_update_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, second_update_data_gas_price);

    let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
    assert_eq!(pending_block.l1_gas_price, second_update_gas_price,);
    assert_eq!(pending_block.l1_data_gas_price, second_update_data_gas_price,);
}

#[tokio::test]
async fn unsuccessful_declare_set_gas_successful_declare() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    ));
    let (contract_class, casm_class_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

    let max_gas = 1e14 as u64;
    let max_gas_price = 1;

    let unsuccessful_declare_tx = predeployed_account
        .declare_v3(Arc::new(contract_class.clone()), casm_class_hash)
        .gas(max_gas)
        .gas_price(max_gas_price)
        .send()
        .await;

    match unsuccessful_declare_tx {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientMaxFee,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    let wei_price = 9e8 as u128;
    let fri_price = 7e8 as u128;
    let gas_price = json!({
        "gas_price_wei": 9e8 as u128,
        "data_gas_price_wei": 8e8 as u128,
        "gas_price_fri": 7e8 as u128,
        "data_gas_price_fri": 6e8 as u128,
    });
    let gas_response = devnet.set_gas_price(&gas_price, true).await.unwrap();
    assert_eq!(gas_response, gas_price);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 1);

    let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
    assert_eq!(
        pending_block.l1_gas_price,
        ResourcePrice { price_in_wei: wei_price.into(), price_in_fri: fri_price.into() }
    );

    let successful_declare_tx = predeployed_account
        .declare_v3(Arc::new(contract_class), casm_class_hash)
        .gas(max_gas)
        .gas_price(max_gas_price)
        .send()
        .await
        .unwrap();
    assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client).await;
}

#[tokio::test]
async fn set_gas_optional_parameters() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(
        latest_block.l1_gas_price,
        ResourcePrice {
            price_in_wei: (u128::from(DEVNET_DEFAULT_GAS_PRICE)).into(),
            price_in_fri: (u128::from(DEVNET_DEFAULT_GAS_PRICE)).into(),
        }
    );

    // set nothing, get initial gas information and assert
    let gas_response = devnet.set_gas_price(&json!({}), false).await.unwrap();
    assert_eq!(
        gas_response,
        json!({
            "gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
            "data_gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
            "gas_price_fri": DEVNET_DEFAULT_GAS_PRICE,
            "data_gas_price_fri": DEVNET_DEFAULT_GAS_PRICE,
        })
    );

    let expected_final_gas_price = json!({
        "gas_price_wei": 9e18 as u128,
        "data_gas_price_wei": 8e18 as u128,
        "gas_price_fri": 7e18 as u128,
        "data_gas_price_fri": 6e18 as u128,
    });

    for (gas_prop, gas_price) in expected_final_gas_price.as_object().unwrap() {
        // Construct the JSON request dynamically based on the parameter
        let optional_gas_request = json!({ gas_prop: gas_price });
        let gas_response = devnet.set_gas_price(&optional_gas_request, true).await.unwrap();
        assert_eq!(&gas_response[gas_prop], gas_price);
    }

    // set nothing, get final gas information and assert
    let gas_response = devnet.set_gas_price(&json!({}), false).await.unwrap();
    assert_eq!(gas_response, expected_final_gas_price);
}
