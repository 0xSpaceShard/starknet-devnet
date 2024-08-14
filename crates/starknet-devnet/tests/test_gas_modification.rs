#![cfg(test)]
pub mod common;

mod gas_modification_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::constants::DEVNET_DEFAULT_GAS_PRICE;
    use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::{Felt, ResourcePrice, StarknetError};
    use starknet_rs_providers::ProviderError;
    use starknet_types::chain_id::ChainId;
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{self, CAIRO_1_CONTRACT_PATH, INTEGRATION_SEPOLIA_HTTP_URL};
    use crate::common::fees::assert_difference_if_validation;
    use crate::common::utils::{
        assert_tx_successful, get_flattened_sierra_contract_and_casm_hash,
        get_simple_contract_in_sierra_and_compiled_class_hash, iter_to_hex_felt, to_hex_felt,
        to_num_as_hex,
    };

    /// Test scenario for gas modification involving simulateTransactions:
    /// 1. Execute simulateTransactions with a declare transaction and check gas fees.
    /// 2. Set the gas values.
    /// 3. Execute simulateTransactions again and check gas fees.
    async fn set_gas_scenario(devnet: BackgroundDevnet, expected_chain_id: &str) {
        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            felt_from_prefixed_hex(expected_chain_id).unwrap(),
            ExecutionEncoding::New,
        );

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

        let max_fee = Felt::ZERO;
        let nonce = Felt::ZERO;

        let signature = account
            .declare_v2(Arc::new(flattened_contract_artifact.clone()), casm_hash)
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_declare_request(false)
            .await
            .unwrap()
            .signature;

        let signature_hex: Vec<String> = iter_to_hex_felt(&signature);
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
                        "signature": signature_hex,
                        "nonce": to_num_as_hex(&nonce),
                        "contract_class": flattened_contract_artifact,
                    }
                ]
            })
        };

        let chain_id = &devnet.send_custom_rpc("starknet_chainId", json!({})).await.unwrap();
        assert_eq!(chain_id, expected_chain_id);

        let params_no_flags = get_params(&[]);
        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags.clone())
            .await
            .unwrap()[0];
        assert_eq!(
            resp_no_flags["fee_estimation"]["gas_price"],
            format!("0x{:x}", DEVNET_DEFAULT_GAS_PRICE)
        );
        assert_eq!(
            resp_no_flags["fee_estimation"]["data_gas_price"],
            format!("0x{:x}", DEVNET_DEFAULT_GAS_PRICE)
        );
        assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0xa7275ca6d3000");

        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation.clone())
            .await
            .unwrap()[0];
        assert_eq!(
            resp_skip_validation["fee_estimation"]["gas_price"],
            format!("0x{:x}", DEVNET_DEFAULT_GAS_PRICE)
        );
        assert_eq!(
            resp_skip_validation["fee_estimation"]["data_gas_price"],
            format!("0x{:x}", DEVNET_DEFAULT_GAS_PRICE)
        );
        assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0xa7247397f6000");

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
            "generate_block": true,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
            "gas_price_wei": wei_price,
            "data_gas_price_wei": wei_price_data,
            "gas_price_fri": 7e18 as u128,
            "data_gas_price_fri": 6e18 as u128,
        });
        assert_eq!(gas_response, &expected_gas_response);

        let chain_id = &devnet.send_custom_rpc("starknet_chainId", json!({})).await.unwrap();
        assert_eq!(chain_id, expected_chain_id);

        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await
            .unwrap()[0];

        assert_eq!(resp_no_flags["fee_estimation"]["gas_price"], format!("0x{:x}", wei_price));
        assert_eq!(
            resp_no_flags["fee_estimation"]["data_gas_price"],
            format!("0x{:x}", wei_price_data)
        );
        assert_eq!(resp_no_flags["fee_estimation"]["overall_fee"], "0x38008384ec45ab780000");

        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await
            .unwrap()[0];
        assert_eq!(
            resp_skip_validation["fee_estimation"]["gas_price"],
            format!("0x{:x}", wei_price)
        );
        assert_eq!(
            resp_skip_validation["fee_estimation"]["data_gas_price"],
            format!("0x{:x}", wei_price_data)
        );
        assert_eq!(resp_skip_validation["fee_estimation"]["overall_fee"], "0x37ff89b813a3e6700000");

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
        set_gas_scenario(devnet, ChainId::Testnet.to_felt().to_hex_string().as_str()).await;
    }

    #[tokio::test]
    async fn set_gas_fork() {
        let cli_args: [&str; 2] = ["--fork-network", INTEGRATION_SEPOLIA_HTTP_URL];
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

        // Sepolia fork gas modification test scenario
        set_gas_scenario(fork_devnet, "0x534e5f494e544547524154494f4e5f5345504f4c4941").await;
    }

    #[tokio::test]
    async fn set_gas_check_blocks() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 0);
        assert_eq!(
            latest_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
                price_in_fri: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
            }
        );

        let wei_price_first_update = 9e18 as u128;
        let fri_price_first_update = 7e18 as u128;
        let gas_request = json!({
            "gas_price_wei": wei_price_first_update,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_fri": fri_price_first_update,
            "data_gas_price_fri": 6e18 as u128,
            "generate_block": false,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
            "gas_price_wei": wei_price_first_update,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_fri": fri_price_first_update,
            "data_gas_price_fri": 6e18 as u128,
        });
        assert_eq!(gas_response, &expected_gas_response);

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 0);

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
                price_in_fri: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
            }
        );

        devnet.create_block().await.unwrap();

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price_first_update),
                price_in_fri: Felt::from(fri_price_first_update),
            }
        );

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 1);
        assert_eq!(
            latest_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
                price_in_fri: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
            }
        );

        let wei_price_second_update = 8e18 as u128;
        let fri_price_second_update = 6e18 as u128;
        let gas_request = json!({
            "gas_price_wei": wei_price_second_update,
            "data_gas_price_wei": 7e18 as u128,
            "gas_price_fri": fri_price_second_update,
            "data_gas_price_fri": 5e18 as u128,
            "generate_block": true,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
            "gas_price_wei": wei_price_second_update,
            "data_gas_price_wei": 7e18 as u128,
            "gas_price_fri": fri_price_second_update,
            "data_gas_price_fri": 5e18 as u128,
        });
        assert_eq!(gas_response, &expected_gas_response);

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 2);
        assert_eq!(
            latest_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price_first_update),
                price_in_fri: Felt::from(fri_price_first_update),
            }
        );

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price_second_update),
                price_in_fri: Felt::from(fri_price_second_update),
            }
        );
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
        let (contract_class, casm_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        let max_gas_fee = Felt::from(1e14 as u128);

        let unsuccessful_declare_tx = predeployed_account
            .declare_v2(Arc::new(contract_class.clone()), casm_class_hash)
            .max_fee(max_gas_fee)
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
        let gas_request = json!({
            "gas_price_wei": 9e8 as u128,
            "data_gas_price_wei": 8e8 as u128,
            "gas_price_fri": 7e8 as u128,
            "data_gas_price_fri": 6e8 as u128,
            "generate_block": true,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
            "gas_price_wei": 9e8 as u128,
            "data_gas_price_wei": 8e8 as u128,
            "gas_price_fri": 7e8 as u128,
            "data_gas_price_fri": 6e8 as u128,
        });
        assert_eq!(gas_response, &expected_gas_response);

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 1);

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price),
                price_in_fri: Felt::from(fri_price),
            }
        );

        let successful_declare_tx = predeployed_account
            .declare_v2(Arc::new(contract_class), casm_class_hash)
            .max_fee(max_gas_fee)
            .send()
            .await
            .unwrap();
        assert_tx_successful(&successful_declare_tx.transaction_hash, &devnet.json_rpc_client)
            .await;
    }

    #[tokio::test]
    async fn set_gas_optional_parameters() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(
            latest_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
                price_in_fri: Felt::from(u128::from(DEVNET_DEFAULT_GAS_PRICE)),
            }
        );

        // set nothing, get initial gas information and assert
        let gas_request = json!({
            "generate_block": false,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
                "gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
                "data_gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
                "gas_price_fri": DEVNET_DEFAULT_GAS_PRICE,
                "data_gas_price_fri": DEVNET_DEFAULT_GAS_PRICE,
        });
        assert_eq!(gas_response, &expected_gas_response);

        let gas_test_data = [
            ("gas_price_wei", 9e18 as u128),
            ("data_gas_price_wei", 8e18 as u128),
            ("gas_price_fri", 7e18 as u128),
            ("data_gas_price_fri", 6e18 as u128),
        ];
        for gas_parameter in gas_test_data.iter() {
            // Construct the JSON request dynamically based on the parameter
            let optional_gas_request = json!({
                gas_parameter.0: gas_parameter.1,
                "generate_block": true,
            });
            let gas_response = &devnet
                .send_custom_rpc("devnet_setGasPrice", optional_gas_request.clone())
                .await
                .unwrap();

            let value = gas_response[gas_parameter.0]
                .as_u64()
                .expect("Failed to get value from JSON response") as u128;
            assert_eq!(value, gas_parameter.1);
        }

        // set nothing, get final gas information and assert
        let gas_request = json!({
            "generate_block": false,
        });
        let gas_response =
            &devnet.send_custom_rpc("devnet_setGasPrice", gas_request.clone()).await.unwrap();
        let expected_gas_response = json!({
            "gas_price_wei": 9e18 as u128,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_fri": 7e18 as u128,
            "data_gas_price_fri": 6e18 as u128,
        });

        assert_eq!(gas_response, &expected_gas_response);
    }
}
