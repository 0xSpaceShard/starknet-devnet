use std::sync::Arc;

use serde_json::json;
use starknet_core::constants::{
    DEVNET_DEFAULT_L1_DATA_GAS_PRICE, DEVNET_DEFAULT_L1_GAS_PRICE, DEVNET_DEFAULT_L2_GAS_PRICE,
};
use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::chain_id::SEPOLIA;
use starknet_rs_core::types::{Felt, ResourcePrice, StarknetError};
use starknet_rs_core::utils::cairo_short_string_to_felt;
use starknet_rs_providers::{Provider, ProviderError};
use starknet_rs_signers::Signer;

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_CONTRACT_PATH, INTEGRATION_SAFE_BLOCK, INTEGRATION_SEPOLIA_HTTP_URL,
};
use crate::common::errors::RpcError;
use crate::common::fees::assert_difference_if_validation;
use crate::common::utils::{
    assert_tx_succeeded_accepted, felt_to_u128, get_flattened_sierra_contract_and_casm_hash,
    get_simple_contract_artifacts, iter_to_hex_felt, to_hex_felt, to_num_as_hex,
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
///
/// Chain ID assertion justified in: https://github.com/0xSpaceShard/starknet-devnet/pull/551#discussion_r1682755696
///
/// Note to test maintainer: the usual way of adapting this test to a new Starknet version is to
/// repeatedly run it and hardcode new hex fee values.
async fn set_gas_scenario(devnet: BackgroundDevnet, expected_chain_id: Felt) {
    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        expected_chain_id,
        ExecutionEncoding::New,
    );

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

    let nonce = Felt::ZERO;

    let declaration = account
        .declare_v3(Arc::new(flattened_contract_artifact.clone()), casm_hash)
        .l1_gas(0)
        .l1_gas_price(0)
        .l1_data_gas(0)
        .l1_data_gas_price(0)
        .l2_gas(0)
        .l2_gas_price(0)
        .nonce(nonce)
        .prepared()
        .unwrap();
    let signature = signer.sign_hash(&declaration.transaction_hash(false)).await.unwrap();

    let zero_bounds = json!({ "max_amount": "0x0", "max_price_per_unit": "0x0" });
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
                    "version": "0x3",
                    "resource_bounds": {
                        "l1_gas": zero_bounds,
                        "l1_data_gas": zero_bounds,
                        "l2_gas": zero_bounds,
                    },
                    "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                    "nonce": to_num_as_hex(&nonce),
                    "contract_class": flattened_contract_artifact,
                    "tip": "0x0",
                    "paymaster_data": [],
                    "account_deployment_data": [],
                    "nonce_data_availability_mode": "L1",
                    "fee_data_availability_mode": "L1",
                }
            ]
        })
    };

    let chain_id = devnet.json_rpc_client.chain_id().await.unwrap();
    assert_eq!(chain_id, expected_chain_id);

    let params_skip_fee_charge = get_params(&["SKIP_FEE_CHARGE"]);
    let resp_no_flags = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_fee_charge.clone())
        .await
        .unwrap()[0];
    assert_eq!(
        resp_no_flags["fee_estimation"]["l1_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L1_GAS_PRICE)
    );
    assert_eq!(
        resp_no_flags["fee_estimation"]["l1_data_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L1_DATA_GAS_PRICE)
    );
    assert_eq!(
        resp_no_flags["fee_estimation"]["l2_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L2_GAS_PRICE)
    );
    assert_eq!(resp_no_flags["transaction_trace"]["execution_resources"]["l1_gas"], 0);
    assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0x99cb411f968000");

    let params_skip_validation_and_fee_charge = get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]);
    let resp_skip_validation = &devnet
        .send_custom_rpc(
            "starknet_simulateTransactions",
            params_skip_validation_and_fee_charge.clone(),
        )
        .await
        .unwrap()[0];

    assert_eq!(
        resp_skip_validation["fee_estimation"]["l1_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L1_GAS_PRICE)
    );
    assert_eq!(
        resp_skip_validation["fee_estimation"]["l1_data_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L1_DATA_GAS_PRICE)
    );
    assert_eq!(
        resp_skip_validation["fee_estimation"]["l2_gas_price"],
        to_hex_felt(&DEVNET_DEFAULT_L2_GAS_PRICE)
    );
    assert_eq!(resp_no_flags["transaction_trace"]["execution_resources"]["l1_gas"], 0);
    assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0x995e1d72370000");

    let should_skip_fee_invocation = true;
    assert_difference_if_validation(
        resp_no_flags,
        resp_skip_validation,
        &sender_address_hex,
        should_skip_fee_invocation,
    );

    let l1_fri_price = 8.5e18 as u128;
    let l1_data_fri_price = 7.5e18 as u128;
    let l2_fri_price = 6.5e18 as u128;
    let gas_request = json!({
        "gas_price_wei": 9e18 as u128,
        "gas_price_fri": l1_fri_price,
        "data_gas_price_wei": 8e18 as u128,
        "data_gas_price_fri": l1_data_fri_price,
        "l2_gas_price_wei": 7e18 as u128,
        "l2_gas_price_fri": l2_fri_price,
    });
    let gas_response = &devnet.set_gas_price(&gas_request, true).await.unwrap();

    assert_eq!(gas_response, &gas_request);

    let chain_id = devnet.json_rpc_client.chain_id().await.unwrap();
    assert_eq!(chain_id, expected_chain_id);

    let resp_no_flags = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_fee_charge)
        .await
        .unwrap()[0];

    assert_eq!(resp_no_flags["fee_estimation"]["l1_gas_price"], to_hex_felt(&l1_fri_price));
    assert_eq!(
        resp_no_flags["fee_estimation"]["l1_data_gas_price"],
        to_hex_felt(&l1_data_fri_price)
    );
    assert_eq!(resp_no_flags["fee_estimation"]["l2_gas_price"], to_hex_felt(&l2_fri_price));
    assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0xe8c077047881faf1800000");

    let resp_skip_validation = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_validation_and_fee_charge)
        .await
        .unwrap()[0];
    assert_eq!(resp_skip_validation["fee_estimation"]["l1_gas_price"], to_hex_felt(&l1_fri_price));
    assert_eq!(
        resp_skip_validation["fee_estimation"]["l1_data_gas_price"],
        to_hex_felt(&l1_data_fri_price)
    );
    assert_eq!(resp_skip_validation["fee_estimation"]["l2_gas_price"], to_hex_felt(&l2_fri_price));
    assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0xe81b4b21fb0b18a2000000");

    assert_difference_if_validation(
        resp_no_flags,
        resp_skip_validation,
        &sender_address_hex,
        should_skip_fee_invocation,
    );
}

#[tokio::test]
async fn set_gas() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // Testnet gas modification test scenario
    set_gas_scenario(devnet, SEPOLIA).await;
}

#[tokio::test]
async fn set_gas_fork() {
    let fork_block = &INTEGRATION_SAFE_BLOCK.to_string();
    let cli_args = ["--fork-network", INTEGRATION_SEPOLIA_HTTP_URL, "--fork-block", fork_block];
    let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

    // Sepolia fork gas modification test scenario
    set_gas_scenario(fork_devnet, cairo_short_string_to_felt("SN_INTEGRATION_SEPOLIA").unwrap())
        .await;
}

#[tokio::test]
async fn set_gas_check_blocks() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let default_gas_price = ResourcePrice {
        price_in_wei: u128::from(DEVNET_DEFAULT_L1_GAS_PRICE).into(),
        price_in_fri: u128::from(DEVNET_DEFAULT_L1_GAS_PRICE).into(),
    };
    let default_data_gas_price = ResourcePrice {
        price_in_wei: u128::from(DEVNET_DEFAULT_L1_DATA_GAS_PRICE).into(),
        price_in_fri: u128::from(DEVNET_DEFAULT_L1_DATA_GAS_PRICE).into(),
    };
    let default_l2_gas_price = ResourcePrice {
        price_in_wei: u128::from(DEVNET_DEFAULT_L2_GAS_PRICE).into(),
        price_in_fri: u128::from(DEVNET_DEFAULT_L2_GAS_PRICE).into(),
    };

    // First update - don't generate new block
    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 0);
    assert_eq!(latest_block.l1_gas_price, default_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, default_data_gas_price);
    assert_eq!(latest_block.l2_gas_price, default_l2_gas_price);

    let first_update_gas_price =
        ResourcePrice { price_in_wei: (9e18 as u128).into(), price_in_fri: (7e18 as u128).into() };
    let first_update_data_gas_price =
        ResourcePrice { price_in_wei: (8e18 as u128).into(), price_in_fri: (6e18 as u128).into() };
    let first_update_l2_gas_price = ResourcePrice {
        price_in_wei: (8.5e18 as u128).into(),
        price_in_fri: (7.5e18 as u128).into(),
    };
    let gas_request = json!({
        "gas_price_wei": felt_to_u128(first_update_gas_price.price_in_wei),
        "data_gas_price_wei": felt_to_u128(first_update_data_gas_price.price_in_wei),
        "l2_gas_price_wei": felt_to_u128(first_update_l2_gas_price.price_in_wei),
        "gas_price_fri": felt_to_u128(first_update_gas_price.price_in_fri),
        "data_gas_price_fri": felt_to_u128(first_update_data_gas_price.price_in_fri),
        "l2_gas_price_fri": felt_to_u128(first_update_l2_gas_price.price_in_fri),

    });
    let gas_response = devnet.set_gas_price(&gas_request, false).await.unwrap();
    assert_eq!(gas_response, gas_request);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 0);

    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    assert_eq!(pre_confirmed_block.l1_gas_price, default_gas_price);
    assert_eq!(pre_confirmed_block.l1_data_gas_price, default_gas_price);

    devnet.create_block().await.unwrap();

    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    assert_eq!(pre_confirmed_block.l1_gas_price, first_update_gas_price);
    assert_eq!(pre_confirmed_block.l1_data_gas_price, first_update_data_gas_price);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 1);
    assert_eq!(latest_block.l1_gas_price, first_update_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, first_update_data_gas_price);

    // Second update - generate new block
    let second_update_gas_price =
        ResourcePrice { price_in_wei: (8e18 as u128).into(), price_in_fri: (6e18 as u128).into() };
    let second_update_data_gas_price =
        ResourcePrice { price_in_wei: (7e18 as u128).into(), price_in_fri: (5e18 as u128).into() };
    let second_update_l2_gas_price = ResourcePrice {
        price_in_wei: (7.5e18 as u128).into(),
        price_in_fri: (6.5e18 as u128).into(),
    };
    let gas_price = json!({
        "gas_price_wei": felt_to_u128(second_update_gas_price.price_in_wei),
        "data_gas_price_wei": felt_to_u128(second_update_data_gas_price.price_in_wei),
        "l2_gas_price_wei": felt_to_u128(second_update_l2_gas_price.price_in_wei),
        "gas_price_fri": felt_to_u128(second_update_gas_price.price_in_fri),
        "data_gas_price_fri": felt_to_u128(second_update_data_gas_price.price_in_fri),
        "l2_gas_price_fri": felt_to_u128(second_update_l2_gas_price.price_in_fri),
    });
    let gas_response = devnet.set_gas_price(&gas_price, true).await.unwrap();
    assert_eq!(gas_response, gas_price);

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 2);
    assert_eq!(latest_block.l1_gas_price, second_update_gas_price);
    assert_eq!(latest_block.l1_data_gas_price, second_update_data_gas_price);
    assert_eq!(latest_block.l2_gas_price, second_update_l2_gas_price);

    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    assert_eq!(pre_confirmed_block.l1_gas_price, second_update_gas_price);
    assert_eq!(pre_confirmed_block.l1_data_gas_price, second_update_data_gas_price);
    assert_eq!(pre_confirmed_block.l2_gas_price, second_update_l2_gas_price);
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
    let (contract_class, casm_class_hash) = get_simple_contract_artifacts();
    let shared_class = Arc::new(contract_class);

    let l1_gas = 0;
    let l1_data_gas = 1000;
    // used l2 gas (pre-calculated, must be at least this, otherwise insufficient resources): 4.3e7
    let l2_gas = 1.1e12 as u64; // l2_balance (1e21) / l2_price (1e9) + a bit to exceed balance

    let unsuccessful_declare_tx = predeployed_account
        .declare_v3(shared_class.clone(), casm_class_hash)
        .l1_gas(l1_gas)
        .l1_data_gas(l1_data_gas)
        .l2_gas(l2_gas)
        .send()
        .await;

    match unsuccessful_declare_tx {
        Err(AccountError::Provider(ProviderError::StarknetError(
            StarknetError::InsufficientAccountBalance,
        ))) => (),
        other => panic!("Unexpected result: {other:?}"),
    };

    let new_l2_fri_price = 9e7 as u128; // approximate upper limit that will pass
    let gas_price = json!({ "l2_gas_price_fri": new_l2_fri_price });
    let gas_response = devnet.set_gas_price(&gas_price, true).await.unwrap();
    assert_eq!(gas_response["l2_gas_price_fri"], json!(new_l2_fri_price));

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(latest_block.block_number, 1);

    let pre_confirmed_block = devnet.get_pre_confirmed_block_with_tx_hashes().await.unwrap();
    assert_eq!(
        pre_confirmed_block.l1_gas_price,
        ResourcePrice {
            price_in_wei: u128::from(DEVNET_DEFAULT_L1_GAS_PRICE).into(),
            price_in_fri: u128::from(DEVNET_DEFAULT_L1_GAS_PRICE).into()
        }
    );
    assert_eq!(
        pre_confirmed_block.l1_data_gas_price,
        ResourcePrice {
            price_in_wei: u128::from(DEVNET_DEFAULT_L1_DATA_GAS_PRICE).into(),
            price_in_fri: u128::from(DEVNET_DEFAULT_L1_DATA_GAS_PRICE).into()
        }
    );
    assert_eq!(
        pre_confirmed_block.l2_gas_price,
        ResourcePrice {
            price_in_wei: u128::from(DEVNET_DEFAULT_L2_GAS_PRICE).into(),
            price_in_fri: new_l2_fri_price.into()
        }
    );

    let successful_declare_tx = predeployed_account
        .declare_v3(shared_class, casm_class_hash)
        .l1_gas(l1_gas)
        .l1_data_gas(l1_data_gas)
        .l2_gas(l2_gas)
        .send()
        .await
        .unwrap();
    assert_tx_succeeded_accepted(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client)
        .await
        .unwrap();
}

#[tokio::test]
async fn set_gas_optional_parameters() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
    assert_eq!(
        latest_block.l1_gas_price,
        ResourcePrice {
            price_in_wei: (u128::from(DEVNET_DEFAULT_L1_GAS_PRICE)).into(),
            price_in_fri: (u128::from(DEVNET_DEFAULT_L1_GAS_PRICE)).into(),
        }
    );

    // set nothing, get initial gas information and assert
    let gas_response = devnet.set_gas_price(&json!({}), false).await.unwrap();
    assert_eq!(
        gas_response,
        json!({
            "gas_price_wei": DEVNET_DEFAULT_L1_GAS_PRICE,
            "data_gas_price_wei": DEVNET_DEFAULT_L1_GAS_PRICE,
            "l2_gas_price_wei": DEVNET_DEFAULT_L2_GAS_PRICE,
            "gas_price_fri": DEVNET_DEFAULT_L1_GAS_PRICE,
            "data_gas_price_fri": DEVNET_DEFAULT_L1_GAS_PRICE,
            "l2_gas_price_fri": DEVNET_DEFAULT_L2_GAS_PRICE,
        })
    );

    let expected_final_gas_price = json!({
        "gas_price_wei": 9e18 as u128,
        "data_gas_price_wei": 8e18 as u128,
        "l2_gas_price_wei": 7.5e18 as u128,
        "gas_price_fri": 7e18 as u128,
        "data_gas_price_fri": 6e18 as u128,
        "l2_gas_price_fri": 5.5e18 as u128,
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
