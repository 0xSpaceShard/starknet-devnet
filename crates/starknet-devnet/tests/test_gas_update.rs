pub mod common;

mod gas_update_tests {
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

    /// Test scenario for gas update involving simulateTransactions:
    /// 1. Execute simulateTransactions with a declare transaction and check gas fees.
    /// 2. Update the gas values.
    /// 3. Execute simulateTransactions again and check gas fees.
    async fn update_gas_scenario(devnet: BackgroundDevnet, expected_chain_id: &str) {
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
        assert_eq!(
            resp_no_flags["fee_estimation"]["overall_fee"],
            format!("0x{:x}", 29406 * 1e11 as u128)
        );

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
        assert_eq!(
            resp_skip_validation["fee_estimation"]["overall_fee"],
            format!("0x{:x}", 29404 * 1e11 as u128)
        );

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == Felt::ZERO,
        );

        let wei_price = 9e18 as u128;
        let wei_price_data = 8e18 as u128;
        let gas_update_request = json!({
            "gas_price_wei": wei_price,
            "data_gas_price_wei": wei_price_data,
            "gas_price_strk": 7e18 as u128,
            "data_gas_price_strk": 6e18 as u128,
            "generate_block": true,
        });
        let updated_gas =
            &devnet.send_custom_rpc("devnet_updateGas", gas_update_request.clone()).await.unwrap();
        let gas_update_response = json!({
            "gas_price_wei": wei_price,
            "data_gas_price_wei": wei_price_data,
            "gas_price_strk": 7e18 as u128,
            "data_gas_price_strk": 6e18 as u128,
        });
        assert_eq!(updated_gas, &gas_update_response);

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
        assert_eq!(
            resp_no_flags["fee_estimation"]["overall_fee"],
            format!("0x{:x}", 264462 * 1e18 as u128)
        );

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
        assert_eq!(
            resp_skip_validation["fee_estimation"]["overall_fee"],
            format!("0x{:x}", 264444 * 1e18 as u128)
        );

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == Felt::ZERO,
        );
    }

    #[tokio::test]
    async fn update_gas() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // Testnet gas update test scenario
        update_gas_scenario(devnet, ChainId::Testnet.to_felt().to_hex_string().as_str()).await;
    }

    #[tokio::test]
    async fn update_gas_fork() {
        let cli_args: [&str; 2] = ["--fork-network", INTEGRATION_SEPOLIA_HTTP_URL];
        let fork_devnet = BackgroundDevnet::spawn_with_additional_args(&cli_args).await.unwrap();

        // Sepolia fork gas update test scenario
        update_gas_scenario(fork_devnet, "0x534e5f494e544547524154494f4e5f5345504f4c4941").await;
    }

    #[tokio::test]
    async fn update_gas_check_blocks() {
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
        let strk_price_first_update = 7e18 as u128;
        let gas_update_request = json!({
            "gas_price_wei": wei_price_first_update,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_strk": strk_price_first_update,
            "data_gas_price_strk": 6e18 as u128,
            "generate_block": false,
        });
        let updated_gas =
            &devnet.send_custom_rpc("devnet_updateGas", gas_update_request.clone()).await.unwrap();
        let gas_update_response = json!({
            "gas_price_wei": wei_price_first_update,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_strk": strk_price_first_update,
            "data_gas_price_strk": 6e18 as u128,
        });
        assert_eq!(updated_gas, &gas_update_response);

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
                price_in_fri: Felt::from(strk_price_first_update),
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
        let strk_price_second_update = 6e18 as u128;
        let gas_update_block_request = json!({
            "gas_price_wei": wei_price_second_update,
            "data_gas_price_wei": 7e18 as u128,
            "gas_price_strk": strk_price_second_update,
            "data_gas_price_strk": 5e18 as u128,
            "generate_block": true,
        });
        let updated_gas = &devnet
            .send_custom_rpc("devnet_updateGas", gas_update_block_request.clone())
            .await
            .unwrap();
        let gas_update_block_response = json!({
            "gas_price_wei": wei_price_second_update,
            "data_gas_price_wei": 7e18 as u128,
            "gas_price_strk": strk_price_second_update,
            "data_gas_price_strk": 5e18 as u128,
        });
        assert_eq!(updated_gas, &gas_update_block_response);

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 2);
        assert_eq!(
            latest_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price_first_update),
                price_in_fri: Felt::from(strk_price_first_update),
            }
        );

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price_second_update),
                price_in_fri: Felt::from(strk_price_second_update),
            }
        );
    }

    #[tokio::test]
    async fn unsuccessful_declare_update_gas_successful_declare() {
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
        let strk_price = 7e8 as u128;
        let gas_update_request = json!({
            "gas_price_wei": 9e8 as u128,
            "data_gas_price_wei": 8e8 as u128,
            "gas_price_strk": 7e8 as u128,
            "data_gas_price_strk": 6e8 as u128,
            "generate_block": true,
        });
        let updated_gas =
            &devnet.send_custom_rpc("devnet_updateGas", gas_update_request.clone()).await.unwrap();
        let gas_update_response = json!({
            "gas_price_wei": 9e8 as u128,
            "data_gas_price_wei": 8e8 as u128,
            "gas_price_strk": 7e8 as u128,
            "data_gas_price_strk": 6e8 as u128,
        });
        assert_eq!(updated_gas, &gas_update_response);

        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();
        assert_eq!(latest_block.block_number, 1);

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(
            pending_block.l1_gas_price,
            ResourcePrice {
                price_in_wei: Felt::from(wei_price),
                price_in_fri: Felt::from(strk_price),
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
    async fn update_gas_optional_parameters() {
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
        let initial_gas_update_request = json!({
            "generate_block": false,
        });
        let gas_data = &devnet
            .send_custom_rpc("devnet_updateGas", initial_gas_update_request.clone())
            .await
            .unwrap();
        let initial_gas_update_response = json!({
                "gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
                "data_gas_price_wei": DEVNET_DEFAULT_GAS_PRICE,
                "gas_price_strk": DEVNET_DEFAULT_GAS_PRICE,
                "data_gas_price_strk": DEVNET_DEFAULT_GAS_PRICE,
        });
        assert_eq!(gas_data, &initial_gas_update_response);

        let gas_update_data = [
            ("gas_price_wei", 9e18 as u128),
            ("data_gas_price_wei", 8e18 as u128),
            ("gas_price_strk", 7e18 as u128),
            ("data_gas_price_strk", 6e18 as u128),
        ];
        for gas_update_parameter in gas_update_data.iter() {
            // Construct the JSON request dynamically based on the parameter
            let optional_gas_update_request = json!({
                gas_update_parameter.0: gas_update_parameter.1,
                "generate_block": true,
            });
            let updated_gas = &devnet
                .send_custom_rpc("devnet_updateGas", optional_gas_update_request.clone())
                .await
                .unwrap();

            let value = updated_gas[gas_update_parameter.0]
                .as_u64()
                .expect("Failed to get value from JSON response") as u128;
            assert_eq!(value, gas_update_parameter.1);
        }

        // set nothing, get final gas information and assert
        let final_gas_data_request = json!({
            "generate_block": false,
        });
        let gas_data = &devnet
            .send_custom_rpc("devnet_updateGas", final_gas_data_request.clone())
            .await
            .unwrap();

        let final_gas_update_response = json!({
            "gas_price_wei": 9e18 as u128,
            "data_gas_price_wei": 8e18 as u128,
            "gas_price_strk": 7e18 as u128,
            "data_gas_price_strk": 6e18 as u128,
        });
        assert_eq!(gas_data, &final_gas_update_response);
    }
}
