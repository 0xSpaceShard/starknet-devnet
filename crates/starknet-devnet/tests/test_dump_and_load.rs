pub mod common;

mod dump_and_load_tests {
    use std::path::Path;
    use std::time;

    use serde_json::json;
    use server::rpc_core::error::ErrorCode::InvalidParams;
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{send_ctrl_c_signal_and_wait, UniqueAutoDeletableFile};

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    use std::sync::Arc;

    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::FieldElement;

    use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

    async fn dump_load_dump_load(mode: &str) {
        let dump_file =
            UniqueAutoDeletableFile::new(("dump_load_dump_load_on_".to_owned() + mode).as_str());

        for _ in 0..2 {
            let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
                "--dump-path",
                &dump_file.path,
                "--dump-on",
                mode,
            ])
            .await
            .expect("Could not start Devnet");

            devnet_dump.create_block().await.unwrap();
            devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

            send_ctrl_c_signal_and_wait(&devnet_dump.process).await;
        }

        let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            mode,
        ])
        .await
        .expect("Could not start Devnet");

        let last_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(last_block.block_number, 4);
    }

    #[tokio::test]
    async fn dump_load_dump_load_on_exit() {
        dump_load_dump_load("exit").await;
    }

    #[tokio::test]
    async fn dump_load_dump_load_on_transaction() {
        dump_load_dump_load("block").await;
    }

    #[tokio::test]
    async fn dump_wrong_cli_parameters_path() {
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            "///",
            "--dump-on",
            "block",
        ])
        .await;

        assert!(devnet_dump.is_err());
    }

    #[tokio::test]
    async fn dump_and_load_blocks_generation_on_demand() {
        let modes = vec!["exit", "block"];

        for mode in modes {
            let dump_file = UniqueAutoDeletableFile::new(
                ("dump_load_dump_load_on_".to_owned() + mode).as_str(),
            );

            let total_iterations = 2;
            for _ in 0..total_iterations {
                let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
                    "--dump-path",
                    &dump_file.path,
                    "--dump-on",
                    mode,
                    "--block-generation-on",
                    "demand",
                ])
                .await
                .expect("Could not start Devnet");

                devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
                devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
                devnet_dump.create_block().await.unwrap();

                send_ctrl_c_signal_and_wait(&devnet_dump.process).await;
            }

            let devnet_load = BackgroundDevnet::spawn_with_additional_args(&[
                "--dump-path",
                &dump_file.path,
                "--dump-on",
                mode,
                "--block-generation-on",
                "demand",
            ])
            .await
            .expect("Could not start Devnet");

            let last_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();
            assert_eq!(last_block.block_number, total_iterations);
            assert_eq!(last_block.transactions.len(), total_iterations as usize);
        }
    }

    #[tokio::test]
    async fn mint_dump_on_transaction_and_load() {
        // dump after transaction
        let dump_file = UniqueAutoDeletableFile::new("dump_on_transaction");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "block",
        ])
        .await
        .expect("Could not start Devnet");
        let mint_tx_hash_1 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_tx_hash_2 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        // load transaction from file and check transaction hash
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");
        let loaded_transaction_1 =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_1).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction_1
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash_1);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction_1:?}");
        }

        let loaded_transaction_2 =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_2).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction_2
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash_2);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction_2:?}");
        }
    }

    #[tokio::test]
    async fn mint_dump_on_exit_and_load() {
        // dump on exit
        let dump_file = UniqueAutoDeletableFile::new("dump_on_exit");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            dump_file.path.as_str(),
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");
        let devnet_dump_pid = devnet_dump.process.id();
        let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

        // load transaction from file and check transaction hash
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");
        let devnet_load_pid = devnet_load.process.id();
        assert_ne!(devnet_dump_pid, devnet_load_pid); // if PID's are different SIGINT signal worked
        let loaded_transaction =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
    }

    #[tokio::test]
    async fn declare_deploy() {
        let dump_file = UniqueAutoDeletableFile::new("dump_declare_deploy");
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "block",
        ])
        .await
        .expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let predeployed_account = Arc::new(predeployed_account);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let deploy_result = contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // load transaction from file and check transactions hashes
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");

        // check declare transaction
        let loaded_declare_v2 = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(declaration_result.transaction_hash)
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Declare(
            starknet_rs_core::types::DeclareTransaction::V2(declare_v2),
        ) = loaded_declare_v2
        {
            assert_eq!(declare_v2.transaction_hash, declaration_result.transaction_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_declare_v2:?}");
        }

        // check deploy transaction
        let loaded_deploy_v2 = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(deploy_result.transaction_hash)
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(deploy_v2),
        ) = loaded_deploy_v2
        {
            assert_eq!(deploy_v2.transaction_hash, deploy_result.transaction_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_deploy_v2:?}");
        }
    }

    #[tokio::test]
    async fn dump_without_transaction() {
        // dump on exit
        let dump_file = UniqueAutoDeletableFile::new("dump_without_transaction");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

        // file should not be created if there are no transactions
        if Path::new(&dump_file.path).exists() {
            panic!(
                "Could find the dump file but there were no transactions to dump {}",
                &dump_file.path
            );
        }
    }

    #[tokio::test]
    async fn dump_endpoint_fail_with_no_mode_set() {
        let devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let rpc_error = devnet_dump.send_custom_rpc("devnet_dump", json!({})).await.unwrap_err();
        assert!(rpc_error.message.contains("Please provide --dump-on mode"));
    }

    #[tokio::test]
    async fn dump_endpoint_fail_with_wrong_request() {
        let devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let rpc_error = devnet_dump
            .send_custom_rpc(
                "devnet_dump",
                json!({
                    "test": ""
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(rpc_error.code, InvalidParams);
    }

    #[tokio::test]
    async fn dump_endpoint_fail_with_wrong_file_name() {
        let dump_file = UniqueAutoDeletableFile::new("dump_wrong_file_name");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let rpc_error = devnet_dump
            .send_custom_rpc(
                "devnet_dump",
                json!({
                    "path": "///"
                }),
            )
            .await
            .unwrap_err();
        assert!(rpc_error.message.contains("I/O error"));
    }

    #[tokio::test]
    async fn load_endpoint_fail_with_wrong_request() {
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let rpc_error = devnet_load
            .send_custom_rpc(
                "devnet_load",
                json!({
                    "test": ""
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(rpc_error.code, InvalidParams);
    }

    #[tokio::test]
    async fn load_endpoint_fail_with_wrong_path() {
        let load_file_name = "load_file_name";
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet_load
            .send_custom_rpc("devnet_load", json!({ "path": load_file_name }))
            .await
            .unwrap_err();
        assert!(result.message.contains("file does not exist"));
    }

    #[tokio::test]
    async fn dump_load_endpoints_transaction_and_state_after_load_is_valid() {
        // check if the dump with the default path "dump_endpoint" works as expected when json body
        // is empty, later check if the dump with the custom path "dump_endpoint_custom_path"
        // works
        let dump_file = UniqueAutoDeletableFile::new("dump_endpoint");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        devnet_dump.send_custom_rpc("devnet_dump", json!({})).await.unwrap();
        assert!(Path::new(&dump_file.path).exists());

        let dump_file_custom = UniqueAutoDeletableFile::new("dump_endpoint_custom_path");
        devnet_dump
            .send_custom_rpc("devnet_dump", json!({ "path": dump_file_custom.path }))
            .await
            .unwrap();
        assert!(Path::new(&dump_file_custom.path).exists());

        // load and re-execute from "dump_endpoint" file and check if transaction and state of the
        // blockchain is valid
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        devnet_load
            .send_custom_rpc("devnet_load", json!({ "path": dump_file.path }))
            .await
            .unwrap();

        let balance_result = devnet_load
            .get_balance_latest(&FieldElement::from(DUMMY_ADDRESS), FeeUnit::WEI)
            .await
            .unwrap();
        assert_eq!(balance_result, DUMMY_AMOUNT.into());

        let loaded_transaction =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
    }

    #[tokio::test]
    async fn mint_and_dump_and_load_on_same_devnet() {
        let dump_file = UniqueAutoDeletableFile::new("dump_set_time");
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-on",
            "exit",
            "--dump-path",
            &dump_file.path,
        ])
        .await
        .unwrap();

        let unit = FeeUnit::WEI;

        devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
        let balance_before_dump =
            devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
        assert_eq!(balance_before_dump, DUMMY_AMOUNT.into());

        devnet.send_custom_rpc("devnet_dump", json!({ "path": dump_file.path })).await.unwrap();

        devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
        let balance_after_dump =
            devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
        assert_eq!(balance_after_dump, balance_before_dump + DUMMY_AMOUNT.into());

        devnet.send_custom_rpc("devnet_load", json!({ "path": dump_file.path })).await.unwrap();

        let balance_after_load =
            devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
        assert_eq!(balance_after_load, balance_before_dump);

        devnet.mint_unit(DUMMY_ADDRESS, DUMMY_AMOUNT, unit).await;
        let balance_after_mint_on_loaded =
            devnet.get_balance_latest(&DUMMY_ADDRESS.into(), unit).await.unwrap();
        assert_eq!(balance_after_mint_on_loaded, balance_after_load + DUMMY_AMOUNT.into());
    }

    #[tokio::test]
    async fn set_time_with_later_block_generation_dump_and_load() {
        let dump_file = UniqueAutoDeletableFile::new("dump_set_time");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        // set time in past without block generation
        let past_time = 1;
        devnet_dump
            .send_custom_rpc(
                "devnet_setTime",
                json!({ "time": past_time, "generate_block": false }),
            )
            .await
            .unwrap();

        // wait 1 second
        tokio::time::sleep(time::Duration::from_secs(1)).await;

        devnet_dump.create_block().await.unwrap();
        devnet_dump.get_latest_block_with_tx_hashes().await.unwrap();

        // dump and load
        send_ctrl_c_signal_and_wait(&devnet_dump.process).await;

        // load and assert
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");

        let latest_block = devnet_load.get_latest_block_with_tx_hashes().await.unwrap();

        assert_eq!(latest_block.block_number, 1);
        assert_eq!(latest_block.timestamp, past_time);
    }
}
